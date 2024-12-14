use std::error::Error;
use std::result::Result;

fn main() -> Result<(), Box<dyn Error>> {
    let out = jelly_uidmng::command("ls", ["-la"])?;
    print!("{}", String::from_utf8_lossy(&out.stdout));
    Ok(())
}
