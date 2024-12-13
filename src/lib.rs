use nix::unistd::{setegid, seteuid, Gid, Uid};
use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::result::Result;

/// Checks if the current effective user ID (euid) is root.
///
/// This function determines whether the current process is running with root privileges
/// by checking the effective user ID.
///
/// # Returns
///
/// * `true` if the effective user ID is root.
/// * `false` otherwise.
///
/// # Examples
///
/// ```
/// fn main() {
///     if jelly_uidmng::is_root() {
///         println!("Running as root");
///     } else {
///         println!("Not running as root");
///     }
/// }
/// ```
pub fn is_root() -> bool {
    Uid::effective().is_root()
}

/// Changes to root.
///
/// This function attempts to change to root mode
/// by checking the effective user ID.
///
/// # Returns
///
/// * `Ok(())` if the effective user ID is successfully changed to root.
/// * `Err(Box<dyn Error>)` otherwise.
///
/// # Examples
///
/// ```
/// fn main() {
///     jelly_uidmng::change_user();
///     if jelly_uidmng::change_root().is_ok() {
///        println!("Changed to root");
///        assert!(jelly_uidmng::is_root());
///     }
///     else {
///        println!("Not changed to root");
///        assert!(!jelly_uidmng::is_root());
///     }
/// }
/// ```
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

/// Changes to user.
///
/// This function attempts to change to user mode
/// by checking the effective user ID.
///
/// # Returns
///
/// * `Ok(())` if the effective user ID is successfully changed to user.
/// * `Err(Box<dyn Error>)` otherwise.
///
/// # Examples
///
/// ```
/// fn main() {
///     if jelly_uidmng::change_root().is_ok() {
///         jelly_uidmng::change_user();
///         println!("Changed to user");
///         assert!(!jelly_uidmng::is_root());
///     }
///     else {
///         jelly_uidmng::change_user();
///         println!("Already user mode");
///         assert!(!jelly_uidmng::is_root());
///     }
/// }
/// ```
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

/// Executes a command with root privileges.
///
/// This function attempts to execute a given command with the provided arguments as root.
/// If the current effective user ID (euid) is not root, it temporarily changes to root,
/// executes the command, and then reverts back to the original user.
///
/// # Arguments
///
/// * `program` - A string slice that holds the name of the program to be executed.
/// * `args` - An iterator over the arguments to pass to the program.
///
/// # Returns
///
/// * `Ok(Output)` containing the output of the executed command.
/// * `Err(Box<dyn Error>)` if an error occurred during the operation.
///
/// # Errors
///
/// This function will return an error in the following cases:
/// * Changing the effective user ID (euid) or group ID (egid) fails.
/// * The command execution fails.
///
/// # Examples
///
/// ```
/// use std::error::Error;
/// use std::process::Output;
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     let output: Output = jelly_uidmng::command_root("ls", ["-l", "/tmp"])?;
///     println!("Command executed successfully: {}", String::from_utf8_lossy(&output.stdout));
///     Ok(())
/// }
/// ```
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

/// Executes a command in user mode.
///
/// This function attempts to execute a given command with the provided arguments in user mode.
/// If the current effective user ID (euid) is root, it temporarily changes to the user specified
/// by the `SUDO_UID` and `SUDO_GID` environment variables, executes the command, and then
/// reverts back to root.
///
/// # Arguments
///
/// * `program` - A string slice that holds the name of the program to be executed.
/// * `args` - An iterator over the arguments to pass to the program.
///
/// # Returns
///
/// * `Ok(Output)` containing the output of the executed command.
/// * `Err(Box<dyn Error>)` if an error occurred during the operation.
///
/// # Errors
///
/// This function will return an error in the following cases:
/// * The `SUDO_UID` or `SUDO_GID` environment variables are not set or invalid.
/// * Changing the effective user ID (euid) or group ID (egid) fails.
/// * The command execution fails.
///
/// # Examples
///
/// ```
/// use std::error::Error;
/// use std::process::Output;
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     let output: Output = jelly_uidmng::command_user("ls", ["-l", "/tmp"])?;
///     println!("Command executed successfully: {}", String::from_utf8_lossy(&output.stdout));
///     Ok(())
/// }
/// ```
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
/// This function uses the `sudo` command and the `cat` utility to write the provided binary data
/// to the specified file. It requires that the executing user has sudo privileges, and the
/// target file is writable with elevated permissions.
///
/// # Arguments
///
/// * `filename` - A string slice that holds the path of the file to be written to.
/// * `data` - A reference to a `&[u8]` containing the binary data to write.
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
/// * The `cat` command fails to write the data to the file.
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
pub fn write_root(filename: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
    if is_root() {
        // root であればそのまま書き込む
        std::fs::write(filename, data)?;
        Ok(())
    } else {
        // 標準入力を `cat` に渡してファイルに書き込む
        let mut child = Command::new("sudo")
            .arg("sh")
            .arg("-c")
            .arg(format!("cat > {}", filename))
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

/// Reads binary data from a file using `sudo` permissions.
///
/// This function uses the `sudo` command and the `cat` utility to read the binary data
/// from the specified file. It requires that the executing user has sudo privileges, and the
/// target file is readable with elevated permissions.
///
/// # Arguments
///
/// * `filename` - A string slice that holds the path of the file to be read.
///
/// # Returns
///
/// * `Ok(Vec<u8>)` containing the binary data read from the file.
/// * `Err(Box<dyn Error>)` if an error occurred during the operation.
///
/// # Errors
///
/// This function will return an error in the following cases:
/// * The `sudo` command fails or is unavailable.
/// * The `cat` command fails to read the data from the file.
/// * The provided file path is invalid or inaccessible with the required permissions.
///
/// # Examples
///
/// ```
/// use std::error::Error;
///
/// fn main() -> Result<(), Box<dyn Error>> {
///     let filename = "/tmp/test_output.txt";
///
///     let data = jelly_uidmng::read_root(filename)?;
///     println!("File read successfully: {:?}", data);
///     Ok(())
/// }
///```
pub fn read_root(filename: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if is_root() {
        // root であればそのまま読み込む
        let data = std::fs::read(filename)?;
        Ok(data)
    } else {
        // `cat` コマンドを使ってファイルを読み込む
        let output = Command::new("sudo").arg("cat").arg(filename).output()?;

        if output.status.success() {
            Ok(output.stdout) // 成功時はデータを返す
        } else {
            Err(format!("Failed to read from file: {}", filename).into()) // エラー時はエラーメッセージを返す
        }
    }
}
