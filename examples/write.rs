use jelly_uidmng as uidmng;
use std::error::Error;
use std::result::Result;

fn main() -> Result<(), Box<dyn Error>> {
    let data = vec![
        72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33, 10,
    ]; // "Hello, World!\n"
    let filename = "/tmp/test_output.txt";

    uidmng::write_root(filename, &data)?;
    println!("File written successfully: {}", filename);

    Ok(())
}
