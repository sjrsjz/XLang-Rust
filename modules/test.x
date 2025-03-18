create_async_func := () -> (n=>0) -> {
    while (n = n + 1; n < 10000) {
        yield n / 2;
    };
    return "success";
};
n:=0;
asyncs := (,);
while(n = n + 1; n <= 1) {
    obj := create_async_func();
    asyncs = asyncs + (obj,);
    async obj();
    await obj;
};
// print(asyncs);
// n:=0;
// while(n = n + 1; n < 1000000){
//     print(valueof asyncs[0])
// };

// // n:=0;
// // while(n = n + 1; n < len(asyncs)){
// //     await asyncs[n]
// // };

// print("done!");
