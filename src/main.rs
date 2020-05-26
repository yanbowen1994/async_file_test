use futures::future::join_all;
use std::time::Instant;
use rand::Rng;
use std::fs::OpenOptions;
use std::io::Write;

mod async_file {
    use async_std::prelude::*;
    use futures::TryFutureExt;
    use async_std::io::{ErrorKind, self, SeekFrom};
    use async_std::fs::OpenOptions;

    pub async fn read_file(file_path: &str, start: &[usize]) -> io::Result<Vec<u8>> {
        let mut file = OpenOptions::new()
            .read(true)
            .open(file_path)
            .await?;

        let mut res = Vec::new();
        for start in start {
            res.push({
                let mut buffer = vec![0; 32];
                file.seek(SeekFrom::Start(*start as u64)).await?;
                file.read(&mut buffer)
                    .map_err(|_| io::Error::new(ErrorKind::InvalidData, "123"))
                    .await?;
                buffer
            })
        }
        let res = res.concat();

        Ok(res)
    }
}

mod no_async {
    use std::io;
    use std::fs::File;
    use positioned_io::ReadAt;

    pub fn read_file(file: &File, pos: usize) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0; 32];
        file.read_at(pos as u64, &mut buffer)?;
        Ok(buffer)
    }
}

fn create_file(path: &str) {
    let f = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(path);
    if let Ok(mut f) = f {
        println!("create file");
        let tmp = (0..=255).collect::<Vec<_>>();
        let story_data: Vec<Vec<u8>> = (0..1 << 22).into_iter()
            .map(|_|tmp.clone())
            .collect();
        let story_data = story_data.concat();

        f.write_all(&story_data).unwrap();
    }
}

fn main() {
    let file_path = "/mnt/tmp3.txt";
    create_file(file_path);

    println!("init random nums");
    let mut rng = rand::thread_rng();
    let mut randoms = vec![0_usize; 1024 * 1024];
    randoms.iter_mut()
        .for_each(|r| *r = rng.gen_range(0, 16777216 * 2));
    println!("init random nums finish");

    let start = Instant::now();

    let f = std::fs::File::open(file_path).unwrap();

    let res1 = randoms.iter()
        .map(|random| {
            no_async::read_file(&f, *random)
        })
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();

    let res1 = res1.concat();
    println!("sync {:?}", Instant::now() - start);

    let start = Instant::now();

    // use tokio;
    // let mut rt = tokio::runtime::Builder::new()
    //     .build()
    //     .unwrap();

    let res = async_std::task::block_on(join_all(
        randoms
            .chunks(1024)
            .into_iter()
            .map(|random| {
                async_file::read_file(file_path, random)
            })
            .collect::<Vec<_>>()
    ));

    let res = res.into_iter()
        .map(|task| {
            task.unwrap()
        })
        .collect::<Vec<Vec<u8>>>();
    let res = res.concat();
    println!("async {:?}", start.elapsed());

    assert_eq!(res, res1);
}