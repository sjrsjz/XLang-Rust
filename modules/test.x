async_func := (n=>0) -> {
    while (n = n + 1; n < 100000) {
        yield n / 2
    };
    re
};

async async_func();

n:=0;
while(n = n + 1; n < 1000){
    print(valueof async_func)
};

await async_func;

print("done", valueof async_func);