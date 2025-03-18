async_func := (n=>0) -> {
    while (n = n + 1; n < 1000) {
        yield n / 2;
    };
    return "success";
};

async async_func();

n:=0;
while(n = n + 1; n < 100){
    print(valueof async_func)
};

await async_func;

print(keyof async_func, valueof async_func);