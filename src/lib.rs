use std::result::Result;
use std::error::Error;
use std::env;
use std::process::{Command, Output, Stdio};
use std::io::Write;
use std::ffi::OsStr;
use nix::unistd::{setegid, seteuid, Gid, Uid};


pub fn is_root() -> bool {
    Uid::effective().is_root()
}

pub fn change_root() -> Result<(), Box<dyn Error>> {
    // uid が root でない場合は変更できない
    if !Uid::current().is_root() {
        return Err("don't have root permission".into());
    }

    // 既に euid が root である場合は何もしない
    if is_root() {
        return Ok(());
    }

    // root に変更する
    seteuid(Uid::from_raw(0))?;
    setegid(Gid::from_raw(0))?;

    Ok(())
}

pub fn change_user() -> Result<(), Box<dyn Error>> {
    if !is_root() {
        return Ok(());
    }

    // "SUDO_UID" と "SUDO_GID" が設定されていない場合はエラー
    let sudo_uid = env::var("SUDO_UID")?;
    let uid = sudo_uid.parse::<u32>()?;
    let uid = Uid::from_raw(uid);
    let sudo_gid = env::var("SUDO_GID")?;
    let gid = sudo_gid.parse::<u32>()?;
    let gid = Gid::from_raw(gid);

    // SUDO_UID が 既に root の場合は変更できない
    if uid.is_root() {
        return Err("Invalid SUDO_UID".into());
    }

    setegid(gid)?;
    seteuid(uid)?;

    Ok(())
}


pub fn command_root<I, S>(program: S, args: I) -> Result<Output, Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    if is_root() {
        // root であればそのまま実行
        Ok(Command::new(program).args(args).output()?)
    } else {
        // userモードの場合
        if change_root().is_ok() {
            // root に変更できた場合はそのまま実行してuserモードに戻す
            let out = Ok(Command::new(program).args(args).output()?);
            change_user()?;
            out
        } else {
            // root でない場合は sudo で実行
            let mut command_args: Vec<S> = vec![program];
            command_args.extend(args);
            Ok(Command::new("sudo").args(command_args).output()?)
        }
    }
}

pub fn command_user<I, S>(program: S, args: I) -> Result<Output, Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    if !is_root() {
        // root でなければそのまま実行
        Ok(Command::new(program).args(args).output()?)
    } else {
        // userに移行して実行
        change_user()?;
        let out = Ok(Command::new(program).args(args).output()?);
        change_root()?;
        out
    }
}


/// Writes binary data to a file using `sudo` permissions.
///
/// This function uses the `sudo` command and the `tee` utility to write the provided binary data
/// to the specified file. It requires that the executing user has sudo privileges, and the 
/// target file is writable with elevated permissions.
///
/// # Arguments
///
/// * `filename` - A string slice that holds the path of the file to be written to.
/// * `data` - A reference to a `Vec<u8>` containing the binary data to write.
///
/// # Returns
///
/// * `Ok(())` if the file was written successfully.
/// * `Err(Box<dyn Error>)` if an error occurred during the operation.
///
/// # Errors
///
/// This function will return an error in the following cases:
/// * The `sudo` command fails or is unavailable.
/// * The `tee` command fails to write the data to the file.
/// * The provided file path is invalid or inaccessible with the required permissions.
///
/// # Examples
///
/// ```
/// use std::error::Error;
/// 
/// fn main() -> Result<(), Box<dyn Error>> {
///     let data = vec![72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33, 10]; // "Hello, World!\n"
///     let filename = "/tmp/test_output.txt";
/// 
///     jelly_uidmng::write_root(filename, &data)?;
///     println!("File written successfully: {}", filename);
///     Ok(())
/// }
///```
pub fn write_root(filename: &str, data: &Vec<u8>) -> Result<(), Box<dyn Error>> {
    if is_root() {
        // root であればそのまま書き込む
        std::fs::write(filename, data)?;
        Ok(())
    }
    else {
        // 標準入力を `tee` に渡してファイルに書き込む
        let mut child = Command::new("sudo")
            .arg("tee")
            .arg(filename)
            .stdin(Stdio::piped())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(data)?; // データを書き込む
        }

        let status = child.wait()?; // プロセスが終了するのを待つ

        if status.success() {
            Ok(()) // 成功時は Ok を返す
        } else {
            Err(format!("Failed to write to file: {}", filename).into()) // エラー時はエラーメッセージを返す
        }
    }
}
