use nix::unistd::{setegid, seteuid, Gid, Uid};
use std::env;
use std::error;
use std::ffi::OsStr;
use std::process::{Command, Output};
use std::result::Result;

pub fn is_root() -> bool {
    Uid::effective().is_root()
}

pub fn change_root() -> Result<(), Box<dyn error::Error>> {
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

pub fn change_user() -> Result<(), Box<dyn error::Error>> {
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

pub fn command_root<I, S>(program: S, args: I) -> Result<Output, Box<dyn error::Error>>
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

pub fn command_user<I, S>(program: S, args: I) -> Result<Output, Box<dyn error::Error>>
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
