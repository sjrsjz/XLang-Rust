#!/bin/bash

# 编译为共享库
# 编译数学库
gcc -shared -fPIC -o clambda_math.so math_lib.c vm_ffi.c -lm