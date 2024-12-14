use std::error::Error;
use std::result::Result;

fn main() -> Result<(), Box<dyn Error>> {
//  jelly_uidmng::set_allow_sudo(false);

    let filename = "/tmp/test_output.txt";

    let data = jelly_uidmng::read_try(filename)?;
    print!("{}", String::from_utf8_lossy(&data));
    Ok(())
}
