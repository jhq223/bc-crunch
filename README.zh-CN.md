# bc-crunch

[English](README.md) | 简体中文

对 BC1、BC3、BC4、BC5 纹理块进行无损压缩的安全 Rust 库，无第三方依赖。
解压直接写入调用者提供的 BC 缓冲区，不经过 RGBA 解码。

## 安装

```toml
[dependencies]
bc-crunch = "0.1"
```

## 用法

```rust
use bc_crunch::{compress, decompress_into, Format};

fn main() -> Result<(), bc_crunch::Error> {
    let blocks = vec![0u8; Format::Bc1.decoded_len(8, 8)?];
    let packed = compress(8, 8, Format::Bc1, &blocks)?;
    let mut restored = vec![0u8; blocks.len()];
    decompress_into(8, 8, Format::Bc1, &packed, &mut restored)?;
    assert_eq!(restored, blocks);
    Ok(())
}
```

## 数据约定

输入、输出的 BC 块按行排列。宽高必须为非零且是 4 的倍数，缓冲区长度必须与尺寸完全一致。
`Format::decoded_len` 在分配前检查尺寸并计算解压后的 BC 数据大小。
图片边缘不足一个块时，传入补齐后的块网格尺寸。BC1、BC4 每块占 8 字节，BC3、BC5 每块占 16 字节。

压缩完整保留 BC 字节，包括端点顺序和选择索引，不给已经压缩的纹理增加画质损失。
部分数据压缩后可能更大；调用者可以在收益不足时保留原始 BC 块。

数据流不含文件头或校验和，容器需提供格式、尺寸和完整性校验。分配输出前应检查大小，解码出错后丢弃部分输出。解码器不能检测到所有数据损坏。

编解码状态属于单次调用，没有全局可变状态或内部线程池，可并行处理独立图片或分块。
编码器的临时存储随块数量增长；解码器模型存储不随图片尺寸增长。

## 格式互通

本库使用区间编码的 BC 数据流，不是 CRN，也不是实验性的 Huffman 格式。
编码器会附加三个算术解码预读字节。如果外部编码器返回的长度不包含这些字节，导入时须显式补三个零字节。
解码器不会读取输入切片以外的数据。容器应记录编解码版本，不应靠原始字节猜测格式。

## 开发

需要 Rust 1.98.1 或更新版本。

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo run --release --example profile_codec -- 1024 1024 bc3 tile.bc
```

测试覆盖四种格式的外部样本、矩形和大尺寸块网格、截断及畸形输入。
计时示例读取不含容器头的 BC 数据，预热后报告五次测量的中位数，并校验每次解压结果。
命令末尾可以追加输出路径，将压缩流保存到文件。

## 许可

Rust 实现采用 [MPL-2.0](./LICENSE)。第三方部分的原始版权和许可声明保留在 [licenses/](licenses/README.md)。

## 鸣谢

基于 [Geolm 的 bc_crunch](https://github.com/Geolm/bc_crunch) 1.5.2，参考提交为
`88f0a344acc1b2ce3cc1a8393f422aa1033c0539`。
算术编码部分参考了 Amir Said、William A. Pearlman 的工作。
