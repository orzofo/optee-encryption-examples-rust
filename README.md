# optee-encryption-examples-rust

> 在optee中基于rust语言实现的一些加解密程序示例

## (一）创建新项目hello_world-rs的流程

仿照teaclave sdk中的example，但是做了一些改动

1. 把teaclave sdk的引用改为了使用git去链接

2. 修改了TA编译的逻辑，因为是在宿主机编译，所以编译命令不一样

### 1.1 初始化目录

找到一个目录，使用以下命令创建一个uuid.txt

```bash

python -c "import uuid; print(uuid.uuid4())" >> uuid.txt && truncate -s 36 uuid.txt

```

### 1.2 Proto

在项目根目录执行以下命令

```bash

# 创建prote crate
cargo new --lib proto
# 进入proto目录
cd proto
# 添加num_enum crate依赖，去掉default的features（因为包含了std，后续所有包都不能依赖std）
cargo add num_enum --no-default-features

```

然后修改`proto/src/lib.rs`，注意要添加`no_std`标识，实际上所有会被TA依赖的包都要手动添加no_std标识，否则rustc编译的时候会默认把std带进去。

```rust

#![no_std]
use num_enum::{FromPrimitive, IntoPrimitive};

#[derive(FromPrimitive, IntoPrimitive, Debug, Copy, Clone)]
#[repr(u32)]
pub enum Command {
    Test,
    Unknown,
}

pub const UUID: &str = &include_str!("../../uuid.txt");

```

### 1.3 Host

执行cargo new host来创建host crate。
然后修改host/Cargo.toml为如下内容：

```rust

[package]
name = "hello_world-rs"
version = "0.1.0"
edition = "2021"

[dependencies]
libc = "0.2.48"
proto = { path = "../proto" }
optee-teec = { git = "https://github.com/apache/incubator-teaclaave-trustzone-sdk.git", branch = "main", default-features = false }

```

添加host/Makefile为如下内容：

```bash

NAME := host
ARCH ?= aarch64

OBJCOPY : aarch64-linux-gnu-objcopy
TARGET := aarch64-unknown-linux-gnu
OUT_DIR := $(CURDIR)/target/$(TARGET)/release

all: host strip

host:
        @cargo build --target $(TARGET) --release

fmt:
        @cargo fmt

strip:
        @$(OBJCOPY) --strip-unneeded $(OUT_DIR)/$(NAME) $(OUTDIR)/$(NAME)

clean:
        @cargo clean

run:
        $(OUT_DIR)/$(NAME)

```

修改host/src/main.rs为如下内容：

```rust

use optee_teec::{Context,Operation, Session, Uuid}:
use optee_teec::ParamNone;
use proto::{Command,UUID};

fn test_run(session: &mut Session) -> optee_teec::Result<()> {
    let mut operation = Operation::new(0, ParamNone, ParamNone, ParamNone, ParamNone)

    session.invoke_command(Command::Test as u32, &mut operation)?
    Ok(())
}

fn main() -> optee_teec::Result<()> {
    let mut ctx = Context::new()?;
    let uuid = Uuid::parse_str(UUID).unwrap();
    println!("uuid:{}",UUID);
    let mut session = ctx.open_session(uuid)?;

    test_run(&mut session)?;

    println!("Success");
    Ok(())
}

```

### 1.4 TA

执行cargo new ta来添加ta create。
修改ta/Cargo.toml为如下内容：

```rust

[package]
name ="ta"
version = "0.1.0"
edition = "2021"

[dependencies]
proto = { path = "../proto" }
optee-utee = {git = "https://github.com/apache/incubator-teaclave-trustzone-sdk.git", branch = "main", default-features = false }
optee-utee-sys = {git = "https://github.com/apache/incubator-teaclave-trustzone-sdk.git", branch = "main", default-features = false }

[build-dependencies]
proto = { path = "../proto" }
optee-utee-build = "0.2.0"

```

添加ta/Makefile为如下内容(与teaclave example大体一致,只是修改了部分内容以使用宿主机编译链)

```bash

UUID?= $(shell cat "../uuid.txt")

TARGET ?= aarch64-unknown-linux-gnu
OBJCOPY := objcopy

TA_SIGN_KEY ?= $(TA_DEV_KIT_DIR)/keys/default_ta.pem
SIGN := $(TA_DEV_KIT_DIR)/scripts/sign_encrypt.py
OUT_DIR_:= $(CURDIR)/target/$(TARGET)/release

all: ta strip sign

ta:
        @cargo build --target $(TARGET) --release --verbose

strip: ta
        @$(OBJCOPY) --strip-unneeded $(OUT_DIR)/ta $(OUT_DIR)/stripped_ta

sign: strip
        @$(SIGN) --uuid $(UUID) --key $(TA_SIGN_KEY) --in $(OUUT_DIR)/stripped_ta --out $(OUT_DIR)/$(UUID).ta
        @echo "SIGN => ${UUID}"

clean:
        @cargo clean

install:
        sudo cp $(OUT_DIR)/$(UUID).ta /usr/lib/optee_arrmtz/$(UUID).ta

```

添加ta/build.rs为如下内容(与teaclave example一致)

```rust

use proto;
use optee_utee_build::{TaConfig, RustEdition, Error};

fn main() -> Result<(), Error> {
    let config = TaConfig::new_default_with_cargo_env(proto::UUID)?;
    optee_utee_build::build(RustEdition::Before2024, config)

}

```

修改ta/src/main.rs为如下内容

```rust

#![no_std]
#![no_main]

use optee_utee::{
    ta_close_session, ta_create, ta_destroy, ta_invoke_command, ta_open_session, trace_println,
};
use optee_utee::{Error, ErrorKind, Parameters, Result};
use proto::Command;

#[ta_create]
fn create() -> Result<()> {
    trace_println!("[+] TA create");
    Ok(())
}

#[ta_open_session]
fn open_session(_params: &mut Parameters) -> Result<()> {
    trace_println!("[+] TA open session");
    Ok(())
}

#[ta_close_session]
fn close_session() {
    trace_println!("[+] TA close session");
}

#[ta_destroy]
fn destroy() {
    trace_println!("[+] TA destroy");
}

#[ta_invoke_command]
fn invoke_command(cmd_id: u32, params: &mut Parameters) -> Result<()> {
    trace_println!("[+] TA invoke command");
    let mut values = unsafe { params.0.as_value().unwrap() };
    match Command::from(cmd_id) {
        Command::IncValue => {
            values.set_a(values.a() + 100);
            Ok(())
        }
        Command::DecValue => {
            values.set_a(values.a() - 100);
            Ok(())
        }
        _ => Err(Error::new(ErrorKind::BadParameters)),
    }
}

include!(concat!(env!("OUT_DIR"), "/user_ta_header.rs"));

```

### 1.5 测试运行

依次执行如下命令测试运行

```bash

#构建TA
make -C ta
# 复制TA
make -C ta install
#构建CA
make -C host
#运行CA
make -C host run

```

## （二）SM4-rs：基于Rust的OP-TEE国密SM4实现流程

### 代码说明

1. **proto crate**:
   - 定义了CA和TA之间的通信协议
   - 包含命令枚举(加密、解密、生成密钥)
   - 定义了des相关的常量(密钥大小、块大小)

2. **host (CA)**:
   - 使用optee-teec库与TA通信
   - 处理用户输入输出

3. **TA**:
   - 实现des加密解密核心逻辑
   - 处理来自CA的请求并返回结果
   - 使用OP-TEE的安全随机数生成器生成密钥
