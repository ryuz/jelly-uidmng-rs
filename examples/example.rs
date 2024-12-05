

use std::error::Error;
use std::result::Result;
use nix::sys::stat::stat;
use jelly_uidmng::*;

fn assert_file_permission(file: &str, root: bool) {
    let stat = stat(file).unwrap();
    if root {
        assert_eq!(stat.st_uid, 0);
        assert_eq!(stat.st_gid, 0);
    } else {
        assert_ne!(stat.st_uid, 0);
        assert_ne!(stat.st_gid, 0);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    if !is_root() {
        println!("not root!");
        command_root("touch", ["touch_root0.txt"])?;
        command_user("touch", ["touch_user0.txt"])?;
        command_root("touch", ["touch_root1.txt"])?;
        command_user("touch", ["touch_user1.txt"])?;
        assert_file_permission("touch_root0.txt", true);
        assert_file_permission("touch_user0.txt", false);
        assert_file_permission("touch_root1.txt", true);
        assert_file_permission("touch_user1.txt", false);
        command_root("rm", ["touch_root0.txt"])?;
        command_user("rm", ["touch_user0.txt"])?;
        command_root("rm", ["touch_root1.txt"])?;
        command_user("rm", ["touch_user1.txt"])?;
        println!("OK");
        return Ok(());
    }

    println!("root!");

    // root 権限のままファイルを生成して Hello と書き込む
    std::fs::write("test_root0.txt", "Hello")?;
    assert_file_permission("test_root0.txt", true);

    // user 権限に移行してファイルを生成して Hello と書き込む
    change_user()?;
    std::fs::write("test_user0.txt", "Hello")?;
    assert_file_permission("test_user0.txt", false);

    // root 権限に戻ってファイルを生成して Hello と書き込む
    change_root()?;
    std::fs::write("test_root1.txt", "Hello")?;
    assert_file_permission("test_root1.txt", true);
    

    command_root("touch", ["touch_root0.txt"])?;
    command_user("touch", ["touch_user0.txt"])?;
    assert_file_permission("touch_root0.txt", true);
    assert_file_permission("touch_user0.txt", false);

    change_user()?;
    command_root("touch", ["touch_root1.txt"])?;
    command_user("touch", ["touch_user1.txt"])?;
    assert_file_permission("touch_root1.txt", true);
    assert_file_permission("touch_user1.txt", false);

    command_root("rm", ["test_root0.txt"])?;
    command_user("rm", ["test_user0.txt"])?;
    command_user("rm", ["test_root1.txt"])?;
    command_root("rm", ["touch_root0.txt"])?;
    command_user("rm", ["touch_user0.txt"])?;
    command_root("rm", ["touch_root1.txt"])?;
    command_user("rm", ["touch_user1.txt"])?;

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

    println!("OK");

    Ok(())
}
