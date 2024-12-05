

use std::error::Error;
use std::result::Result;
use jelly_uidmng::*;

fn main() -> Result<(), Box<dyn Error>> {
    if !is_root() {
        println!("not root!");
        command_root("touch", ["touch_root0.txt"])?;
        command_user("touch", ["touch_user0.txt"])?;
        command_root("touch", ["touch_root1.txt"])?;
        command_user("touch", ["touch_user1.txt"])?;
        return Ok(());
    }

    // root 権限のままファイルを生成して Hello と書き込む
    std::fs::write("test_root0.txt", "Hello")?;

    // user 権限に移行してファイルを生成して Hello と書き込む
    change_user()?;
    std::fs::write("test_user0.txt", "Hello")?;

    // root 権限に戻ってファイルを生成して Hello と書き込む
    change_root()?;
    std::fs::write("test_root1.txt", "Hello")?;

    command_root("touch", ["touch_root0.txt"])?;
    command_user("touch", ["touch_user0.txt"])?;

    change_user()?;
    command_root("touch", ["touch_root1.txt"])?;
    command_user("touch", ["touch_user1.txt"])?;

    change_root()?;
    assert!(is_root());
    change_user()?;
    assert!(!is_root());
    change_root()?;
    assert!(is_root());
    change_user()?;
    assert!(!is_root());
    change_root()?;
    assert!(is_root());
    change_user()?;
    assert!(!is_root());

    Ok(())
}
