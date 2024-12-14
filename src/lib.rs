use nix::unistd::{setegid, seteuid, Gid, Uid};
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::result::Result;
use std::sync::atomic::{AtomicBool, Ordering};

static ALLOW_SUDO: AtomicBool = AtomicBool::new(false);

/// Sets whether the use of sudo is allowed.
pub fn set_allow_sudo(value: bool) {
    ALLOW_SUDO.store(value, Ordering::SeqCst);
}

/// Returns whether the use of sudo is allowed.
pub fn allow_sudo() -> bool {
    ALLOW_SUDO.load(Ordering::SeqCst)
}

/// Checks if the current effective user ID (euid) is root.
pub fn is_root() -> bool {
    Uid::effective().is_root()
}

/// Checks if the current real user ID (uid) is root.
pub fn has_root() -> bool {
    Uid::current().is_root()
}

/// Changes to root.
pub fn change_root() -> Result<(), Box<dyn Error>> {
    // root 権限を保有していないと変更できない
    if !has_root() {
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

/// Changes to user.
pub fn change_user() -> Result<(), Box<dyn Error>> {
    // 既に euid が 非root である場合は何もしない
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

/// Executes a command with the given program and arguments.
pub fn command<I, S>(program: S, args: I) -> Result<Output, Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    // コマンド実行して結果を返す
    Ok(Command::new(program).args(args).output()?)
}

/// Executes a command with `sudo` using the given program and arguments.
pub fn command_sudo<I, S>(program: S, args: I) -> Result<Output, Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command_args: Vec<S> = vec![program];
    command_args.extend(args);
    Ok(Command::new("sudo").args(command_args).output()?)
}

/// Executes a command in user mode.
pub fn command_user<I, S>(program: S, args: I) -> Result<Output, Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    if !is_root() {
        // root でなければそのまま実行
        command(program, args)
    } else {
        // userに移行して実行
        change_user()?;
        let result = command(program, args);
        change_root()?;
        result
    }
}

/// Executes a command with root privileges.
pub fn command_root<I, S>(program: S, args: I) -> Result<Output, Box<dyn Error>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    if is_root() {
        // root であればそのまま実行
        command(program, args)
    } else {
        // userモードの場合
        if change_root().is_ok() {
            // root に変更できた場合はそのまま実行してuserモードに戻す
            let result = command(program, args);
            change_user()?;
            result
        } else {
            if allow_sudo() {
                // root に変更できない場合は sudo で実行
                command_sudo(program, args)
            } else {
                Err("don't have root permission".into())
            }
        }
    }
}

/// Executes a command and tries to use root permissions if the initial execution fails.
pub fn command_try<I, S>(program: S, args: I) -> Result<Output, Box<dyn Error>>
where
    I: IntoIterator<Item = S> + Clone,
    S: AsRef<OsStr> + Clone,
{
    let result = command(program.clone(), args.clone());
    if let Ok(output) = &result {
        if output.status.success() {
            return result;
        }
    }

    if !is_root() {
        command_root(program, args)
    } else {
        result
    }
}

/// Writes binary data to a file.
pub fn write(filename: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
    std::fs::write(filename, data)?;
    Ok(())
}

/// Reads binary data from a file.
pub fn write_sudo(filename: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
    // 標準入力を `cat` に渡してファイルに書き込む
    let mut child = Command::new("sudo")
        .arg("sh")
        .arg("-c")
        .arg(format!("cat > {}", filename))
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(data)?; // データを書き込む
    } else {
        return Err("Failed to write to file".into());
    }

    // プロセスが終了するのを待つ
    let status = child.wait()?;
    if status.success() {
        Ok(()) // 成功時は Ok を返す
    } else {
        Err(format!("Failed to write to file: {}", filename).into()) // エラー時はエラーメッセージを返す
    }
}

/// Writes binary data to a file using user permissions.
pub fn write_user(filename: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
    if !is_root() {
        write(filename, data)
    } else {
        change_user()?;
        let result = write(filename, data);
        change_root()?;
        result
    }
}

/// Writes binary data to a file using `sudo` permissions.
pub fn write_root(filename: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
    if is_root() {
        // root であればそのまま書き込む
        write(filename, data)
    } else {
        if has_root() {
            change_root()?;
            let result = write(filename, data);
            change_root()?;
            result
        } else {
            if allow_sudo() {
                write_sudo(filename, data)
            } else {
                Err("don't have root permission".into())
            }
        }
    }
}

/// Writes binary data to a file and tries to use root permissions if the initial write fails.
pub fn write_try(filename: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
    let result = write(filename, data);
    if result.is_err() && !is_root() {
        write_root(filename, data)
    } else {
        result
    }
}

/// Reads binary data from a file.
pub fn read(filename: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let data = std::fs::read(filename)?;
    Ok(data)
}

/// Reads binary data from a file using user permissions.
pub fn read_sudo(filename: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    // `cat` コマンドを使ってファイルを読み込む
    let output = command_sudo("cat", [filename])?;
    if output.status.success() {
        Ok(output.stdout) // 成功時はデータを返す
    } else {
        Err(format!("Failed to read from file: {}", filename).into()) // エラー時はエラーメッセージを返す
    }
}

/// Reads binary data from a file using user permissions.
pub fn read_user(filename: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if !is_root() {
        read(filename)
    } else {
        change_user()?;
        let result = read(filename);
        change_root()?;
        result
    }
}

/// Reads binary data from a file using `sudo` permissions.
pub fn read_root(filename: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if is_root() {
        // root であればそのまま読み込む
        read(filename)
    } else {
        if has_root() {
            change_root()?;
            let result = read(filename);
            change_root()?;
            result
        } else {
            if allow_sudo() {
                read_sudo(filename)
            } else {
                Err("don't have root permission".into())
            }
        }
    }
}

/// Reads binary data from a file and tries to use root permissions if the initial read fails.
pub fn read_try(filename: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let result = read(filename);
    if result.is_err() && !is_root() {
        read_root(filename)
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::sys::stat::stat;
    use std::error::Error;
    use std::process::Output;

    fn assert_file_permission(file: &str, root: bool) {
        println!("check file permission : {}", file);
        let stat = stat(file).unwrap();
        if root {
            assert_eq!(stat.st_uid, 0);
            assert_eq!(stat.st_gid, 0);
        } else {
            assert_ne!(stat.st_uid, 0);
            assert_ne!(stat.st_gid, 0);
        }
    }

    #[test]
    fn test_command() -> Result<(), Box<dyn Error>> {
        if !is_root() {
            set_allow_sudo(true);
            command_root("touch", ["/tmp/touch_root0.txt"])?;
            command_user("touch", ["/tmp/touch_user0.txt"])?;
            command_root("touch", ["/tmp/touch_root1.txt"])?;
            command_user("touch", ["/tmp/touch_user1.txt"])?;
            assert_file_permission("/tmp/touch_root0.txt", true);
            assert_file_permission("/tmp/touch_user0.txt", false);
            assert_file_permission("/tmp/touch_root1.txt", true);
            assert_file_permission("/tmp/touch_user1.txt", false);
            command_root("rm", ["/tmp/touch_root0.txt"])?;
            command_user("rm", ["/tmp/touch_user0.txt"])?;
            command_root("rm", ["/tmp/touch_root1.txt"])?;
            command_user("rm", ["/tmp/touch_user1.txt"])?;
        }
        return Ok(());
    }

    #[test]
    fn test_command_sudo() -> Result<(), Box<dyn Error>> {
        let output: Output = command_sudo("echo", ["Hello, world!"])?;
        println!("Output: {:?}", output);
        Ok(())
    }

    #[test]
    fn test_write_user() -> Result<(), Box<dyn Error>> {
        let file_name = "/tmp/test_write_user.txt";
        let write_data = vec![
            72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33, 10,
        ]; // "Hello, World!\n"

        write_user(&file_name, &write_data)?;
        assert_file_permission(&file_name, false);
        let read_data = read_user(&file_name)?;
        assert_eq!(write_data, read_data);

        command_try("rm", [file_name])?;
        Ok(())
    }

    #[test]
    fn test_write_root() -> Result<(), Box<dyn Error>> {
        set_allow_sudo(true);

        let file_name = "/tmp/test_write_root.txt";
        let write_data = vec![
            72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33, 10,
        ]; // "Hello, World!\n"
        write_root(&file_name, &write_data)?;
        assert_file_permission(&file_name, true);
        command_try("chmod", ["700", &file_name])?;

        let result = read_user(&file_name);
        print!("{:?}", result);
        assert!(result.is_err());

        let result = read_root(&file_name);
        assert!(result.is_ok());
        let read_data = result.unwrap();
        assert_eq!(write_data, read_data);

        let result = read_sudo(&file_name);
        assert!(result.is_ok());
        let read_data = result.unwrap();
        assert_eq!(write_data, read_data);

        let result = read_try(&file_name);
        assert!(result.is_ok());
        let read_data = result.unwrap();
        assert_eq!(write_data, read_data);

        command_try("rm", [file_name])?;

        Ok(())
    }

    #[test]
    fn test_set_allow_sudo() -> Result<(), Box<dyn Error>> {
        if !has_root() && !is_root() {
            let file_name = "/tmp/test_root_file.txt";
            let write_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
            set_allow_sudo(true);
            write_root(&file_name, &write_data)?;
            assert_file_permission(&file_name, true);
            command_try("chmod", ["700", &file_name])?;

            set_allow_sudo(false);
            let result = read_root(&file_name);
            assert!(result.is_err());
            let result = read_try(&file_name);
            assert!(result.is_err());

            set_allow_sudo(true);
            let result = read_root(&file_name);
            assert!(result.is_ok());
            let read_data = result.unwrap();
            assert_eq!(write_data, read_data);

            let result = read_try(&file_name);
            assert!(result.is_ok());
            let read_data = result.unwrap();
            assert_eq!(write_data, read_data);

            command_try("rm", [file_name])?;
        }
        Ok(())
    }
}
