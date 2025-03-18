fibo := (n=>0) -> {
    if (n==0 or n==1) {return 1};
    return fibo(n-1)+fibo(n-2);
};

print(fibo(20));