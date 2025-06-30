# 开源操作系统训练营第三阶段——ArceOS



本仓库为 [ArceOS](https://github.com/arceos-org/arceos) 的一个剪裁版本，提供了更多初始化的组件与可用于训练的题目，作为开源操作系统第三阶段的训练题目。

## WIP

体验式实验1-5

1
在内核写程序：打印，axhal、axstd

2
硬件机制：中断与虚拟地址

3
软件机制：panic、动态分配与多执行流

4
启动：从内核入口到main函数

5
宏内核：特权级与syscall

代码实验6-8

6
延迟加载
考察系统调用、异常中断、地址空间

7
包含信号和锁的调度
考察进程、线程、调度器

8
vfs、映射文件、驱动(待定)
考察文件系统、驱动(


## 目录结构

- arceos/：ArceOS 内核源码，它与上游[主线版本](https://github.com/arceos-org/arceos)有所差距，旨在通过剪裁版本让同学们更好理解代码
- course/：ArceOS 教学资料，配合[第三阶段课程](https://opencamp.cn/os2edu/camp/2025spring/stage/3?tab=video)进行学习
- crates/：ArceOS 所依赖并且由我们手动修改的模块，这里仅包括 kernel_guard 一个手动修改的模块
- scripts/：评测脚本，其中 `total-test.sh` 代表执行所有测试，其他脚本分别执行一个测例
- challenges/：挑战题目说明，具体评测脚本位于本仓库 [lab1 分支](https://github.com/LearningOS/2025s-oscamp-stage3/tree/lab1)



## 环境配置

可以参考执行如下命令：

```shell
sudo apt-get update 
sudo apt-get install -y \
  wget \
  xxd \
  curl \
  gcc \
  g++ \
  make \
  libclang-dev \
  qemu-system-misc \
  bash \
  sudo \
  git \
  dosfstools \
  build-essential \
  pkg-config \
  libssl-dev \
  libz-dev \
  libclang-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
cargo install cargo-binutils

mkdir -p /opt/musl && cd /opt/musl
wget https://musl.cc/aarch64-linux-musl-cross.tgz
wget https://musl.cc/riscv64-linux-musl-cross.tgz
wget https://musl.cc/x86_64-linux-musl-cross.tgz
tar zxf aarch64-linux-musl-cross.tgz
tar zxf riscv64-linux-musl-cross.tgz
tar zxf x86_64-linux-musl-cross.tgz

qemu-system-riscv64 --version
source $HOME/.cargo/env
```



## 评测方式

### ArceOS 训练题

在`main`分支根目录下执行：

```shell
./scripts/total-test.sh > tmp.txt
```

此时会对`scripts`下所有脚本进行评测，并将结果输出到 `tmp.txt` 中。每一个评测脚本 100 分，通过即可获得满分。

### 挑战题

请切换到`lab1`分支，执行

```sh
./verify_lab1.sh > tmpa.txt
```

此时会对挑战题进行评测，并将结果输出到 `tmpa.txt` 中。

关于挑战题的评分逻辑，详见[challenge](./challenges)。

