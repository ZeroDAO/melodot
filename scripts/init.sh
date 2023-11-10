#!/bin/bash

# 安装后的标记文件路径
INSTALL_MARKER_FILE="$HOME/.sqlite_installed_marker"

# 检查标记文件是否存在来判断是否已安装
check_sqlite_installed() {
    [[ -f "$INSTALL_MARKER_FILE" ]]
}

# 安装函数
install_debian() {
    sudo apt-get update
    sudo apt-get install -y libsqlite3-dev
}

install_redhat() {
    sudo yum install -y sqlite-devel
}

install_arch() {
    sudo pacman -Sy sqlite
}

# 安装依赖并创建标记文件
install_dependencies() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        case $ID in
            ubuntu|debian)
                install_debian
                ;;
            fedora|centos|rhel)
                install_redhat
                ;;
            arch|manjaro)
                install_arch
                ;;
            *)
                echo "Unsupported operating system: $ID"
                exit 1
                ;;
        esac
        touch "$INSTALL_MARKER_FILE"
    else
        echo "Cannot determine the operating system."
        exit 1
    fi
}

# 检查是否需要安装依赖
if ! check_sqlite_installed; then
    echo "SQLite not installed. Installing dependencies..."
    install_dependencies
else
    echo "All required dependencies are already installed."
fi
