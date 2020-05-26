use futures::future::join_all;
use std::time::Instant;
use rand::Rng;
use std::fs::OpenOptions;
use std::io::Write;

mod file {
    use async_std::fs::File;
    use async_std::prelude::*;
    use futures::TryFutureExt;
    use async_std::io::{ErrorKind, self, SeekFrom};

    pub async fn read_file(path: &str, start: usize) -> io::Result<Vec<u8>> {
        File::open(path)
            .and_then(|mut file| async move {
                let mut buffer = vec![0; 32];
                file.seek(SeekFrom::Start(start as u64)).await?;
                file.read(&mut buffer)
                    .map_err(|_|io::Error::new(ErrorKind::InvalidData, "123"))
                    .await?;
                Ok(buffer)
            }).await
    }
}

mod no_async {
    use std::io;
    use std::fs::File;
    use positioned_io::ReadAt;

    pub fn read_file(path: &str, pos: usize) -> io::Result<Vec<u8>> {
        let file = File::open(path)?;
        let mut buffer = vec![0; 32];
        file.read_at(pos as u64, &mut buffer)?;
        Ok(buffer)
    }
}

fn create_file(path: &str) {
    let f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .read(true)
        .open(path);
    if let Ok(mut f) = f {
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

    let mut rng = rand::thread_rng();
    let mut randoms = vec![0_usize; 1024];
    randoms.iter_mut()
        .for_each(|r| *r = rng.gen_range(0, 16777216 * 2));

    let start = Instant::now();

    let res1 = randoms.iter()
        .map(|random| {
            no_async::read_file(file_path, *random)
        })
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();

    println!("{:?}", Instant::now() - start);

    let start = Instant::now();
    let res = async_std::task::block_on(
        join_all(
            randoms.into_iter()
                .map(move |random| {
                    file::read_file(file_path, random)
                })
                .collect::<Vec<_>>()));
    let res = res.into_iter()
        .map(|task| {
            task.unwrap()
        })
        .collect::<Vec<Vec<u8>>>();
    println!("{:?}", start.elapsed());

    assert_eq!(res, res1);
}