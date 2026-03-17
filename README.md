# API2CLI - OpenAPI to CLI Framework

> 将 RESTful API 快速转为 CLI 工具的 Rust 框架

## 核心特性

- ✅ **动态生成** - 运行时解析 OpenAPI Spec，无需编译
- ✅ **多格式支持** - OpenAPI 3.x / Swagger 2.0，JSON/YAML
- ✅ **自动 Auth** - Bearer Token / API Key / OAuth
- ✅ **Shell 补全** - 自动生成 completions

## 快速开始

```rust
use api2cli::Api2Cli;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api2cli = Api2Cli::new(
        "https://api.example.com/openapi.json",
        Some("your-token".to_string()),
    )?;
    
    let generator = api2cli.generate_cli()?;
    
    // 导出为 shell 脚本
    println!("{}", generator.export_shell());
    
    // 或导出为 Rust clap 代码
    println!("{}", generator.export_rust_clap());
    
    Ok(())
}
```

## 输出示例

### Shell 脚本

```bash
#!/bin/bash
# Auto-generated CLI from OpenAPI spec

# GET /users - Get all users
# Usage: api2cli get-users

# GET /users/{id} - Get user by ID
# Usage: api2cli get-users-id --[id]

# POST /users - Create a new user
# Usage: api2cli create-users --[body]
```

### Rust Clap 代码

```rust
use clap::{Arg, Command};

pub fn build_cli() -> Command {
    Command::new("api2cli")
        .subcommand(Command::new("get-users")
            .about("Get all users")
        )
        .subcommand(Command::new("get-users-id")
            .about("Get user by ID")
            .arg(Arg::new("--id").required(true))
        )
}
```

## 架构

```
api2cli/
├── src/
│   ├── lib.rs       # 主入口
│   ├── spec.rs      # OpenAPI Spec 解析
│   ├── generator.rs # CLI 命令生成
│   ├── runtime.rs   # HTTP 客户端
│   └── main.rs     # Demo Binary
├── Cargo.toml
└── README.md
```

## 运行 Demo

```bash
cd api2cli
API_TOKEN=your-token cargo run -- "https://api.example.com/openapi.json"
```

## 下一步

- [ ] 集成 clap derive 直接生成可运行 CLI
- [ ] 支持更多 auth 方式 (OAuth, API Key)
- [ ] 生成 shell/bash/zsh completions
- [ ] 交互式模式 (类似 restish)
- [ ] Middleware 支持 (logging, retry, rate limit)

## 竞品

- [restish](https://github.com/davecgh/restish) - Go 实现，交互式 REST CLI
- [Kinopen](https://github.com/霜js/kinopen) - Python，OpenAPI → CLI
- [swagger2cli](https://github.com/safe-waters/swagger2cli) - Go
