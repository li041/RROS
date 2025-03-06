- [git repo](https://github.com/linux-test-project/ltp)
- [documentation](https://linux-test-project.readthedocs.io/en/latest/users/quick_start.html)

```
# 对单一的syscall编译结束后, 对所有可执行文件进行批处理
find . -type f -exec file {} \; | grep "ELF 64-bit" | cut -d: -f1
```