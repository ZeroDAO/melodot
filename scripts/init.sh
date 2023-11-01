#!/bin/bash

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
else
    echo "Cannot determine the operating system."
    exit 1
fi
