use std::{
    env, fs,
    io::{self, Write},
    process::Command,
    thread,
    time::Duration,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("sleep") => {
            let millis = args
                .next()
                .ok_or("missing sleep duration")?
                .parse::<u64>()?;
            thread::sleep(Duration::from_millis(millis));
        }
        Some("spawn-once") => {
            let executable = env::current_exe()?;
            match Command::new(executable).args(["sleep", "500"]).status() {
                Ok(status) if status.success() => println!("spawned-ok"),
                Ok(status) => println!("spawn-blocked-status:{status}"),
                Err(error) => println!("spawn-blocked-error:{error}"),
            }
        }
        Some("tree-parent") => {
            let marker = args.next().ok_or("missing marker path")?;
            let executable = env::current_exe()?;
            let child = Command::new(executable)
                .args(["tree-child", marker.as_str()])
                .spawn()?;
            println!("tree-child-started:{}", child.id());
            io::stdout().flush()?;
            thread::sleep(Duration::from_secs(5));
        }
        Some("tree-child") => {
            let marker = args.next().ok_or("missing marker path")?;
            thread::sleep(Duration::from_millis(700));
            fs::write(marker, b"survived")?;
        }
        Some("allocate") => {
            let target = args
                .next()
                .ok_or("missing allocation target")?
                .parse::<usize>()?;
            const CHUNK: usize = 8 * 1024 * 1024;
            let mut chunks = Vec::<Vec<u8>>::new();
            let mut allocated = 0_usize;
            while allocated < target {
                let size = CHUNK.min(target - allocated);
                let mut chunk = Vec::new();
                if let Err(error) = chunk.try_reserve_exact(size) {
                    println!("memory-blocked:{error}");
                    return Ok(());
                }
                chunk.resize(size, 0xA5);
                std::hint::black_box(&chunk);
                chunks.push(chunk);
                allocated = allocated.saturating_add(size);
            }
            println!("memory-allocated:{allocated}");
        }
        Some(other) => return Err(format!("unknown fixture mode: {other}").into()),
        None => return Err("missing fixture mode".into()),
    }
    Ok(())
}
