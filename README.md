# UID manager

## 概要

IoT 開発では、しばしハードウェア制御などで root 権限が無いと行えない操作が発生します。

特に単体テストやプロトタイピングで手っ取り早く root 権限で実行する事もあると思います。

一方で、root で実行してしまうと、そこで作成したファイルなども所有者が root になってしまい面倒なケースもあります。

そこで

- sudo で起動したあと、必要に応じて seteuid で権限を変更する
- ユーザー権限で起動した後、 必要に応じて sudo で特権機能を使う

などを行う事が考えられます。

このような事を少し楽に行う為の機能を集めたものです。

なお、sudo 実行時の環境変数 SUDO_UID と SUDO_GID をユーザーIDとして利用します。

## 使い方

関数名は概ね下記のルールです。

- xxxx_root() : root権限での実行を行う
- xxxx_user() : user権限での実行を行う
- xxxx_try() : 現在の権限で実行して、失敗したら root 権限に格上げして試みる


### 権限変更

change_root() や change_user() などで権限を変更します。


### コマンド実行

command_root()、command_user()、command_try() など、指定した権限での実行を試みます。

```rust
use std::error::Error;
use std::result::Result;
use jelly_uidmng as uidmng;
fn main() -> Result<(), Box<dyn Error>> {
    // 必要なら sudo を使う事を許す
    uidmng::set_allow_sudo(true);

    // root権限で ls を実行
    let out = uidmng::command_root("ls", ["-la". "/lib/firmware"])?;
    print!("{}", String::from_utf8_lossy(&out.stdout));

    Ok(())
}
```

### ファイル書き込み

write_root()、write_user()、write_try() など、指定した権限でのファイル書き込みを試みます。

```rust
use std::error::Error;
use std::result::Result;
use jelly_uidmng as uidmng;

fn main() -> Result<(), Box<dyn Error>> {
    uidmng::write_root("sys/class/gpio/gpio18/value", "1".as_bytes())?;
    Ok(())
}
```

### ファイル読み込み

read_root()、read_user()、read_try() など、指定した権限でのファイル書き込みを試みます。

```rust
use std::error::Error;
use std::result::Result;
use jelly_uidmng as uidmng;

fn main() -> Result<(), Box<dyn Error>> {
    uidmng::set_allow_sudo(true);

    let filename = "/configfs/device-tree/overlays/full/status";

    let data = uidmng::read_try(filename)?;
    print!("{}", String::from_utf8_lossy(&data));
    Ok(())
}

```


