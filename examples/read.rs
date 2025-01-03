use jelly_uidmng as uidmng;
use std::error::Error;
use std::result::Result;

fn main() -> Result<(), Box<dyn Error>> {
    uidmng::set_allow_sudo(true);

    let filename = "/tmp/test_output.txt";

    let data = uidmng::read_try(filename)?;
    print!("{}", String::from_utf8_lossy(&data));
    Ok(())
}
