#!/bin/bash

# 编译为共享库
gcc -shared -fPIC -o libvm_ffi.so vm_ffi.c -Wall
echo "Shared library built: libvm_ffi.so"
