#!/bin/bash
###
 # @Author: peter
 # @Date: 2025-03-06 21:22:40
 # @LastEditors: peter
 # @LastEditTime: 2025-03-06 21:57:18
 # @FilePath: /RROS/ext4_create.sh
 # @Description: create a none ext4 file system
 # 
 # Copyright (c) 2025 by ${git_name_email}, All Rights Reserved. 
### 

# 设置镜像文件名和大小
IMAGE_NAME="empty_ext4.img"
IMAGE_SIZE="1000M"  # 可以更改大小，如 1G

# 创建空的镜像文件
dd if=/dev/zero of=$IMAGE_NAME bs=1M count=${IMAGE_SIZE/M/} status=progress

# 格式化为 ext4 文件系统
mkfs.ext4 $IMAGE_NAME

# 显示镜像信息
echo "已创建 ext4 镜像: $IMAGE_NAME"
ls -lh $IMAGE_NAME
