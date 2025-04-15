try_catch := boundary (@dynamic (__stdlib_root!) -> dyn import (__stdlib_root + "/try_catch.xbc"))();
builtins := boundary (@dynamic (__stdlib_root!) -> dyn import (__stdlib_root + "/builtins.xbc"))();

match_alias := (cases?, __builtins__ => builtins, __try_catch__ => try_catch) -> {
    return (x?, __cases__ => cases, __builtins__!, __try_catch__!) -> {
        n := 0;
        alias := aliasof x;
        while (n < __builtins__.len(__cases__)) {
            if ((keyof __cases__[n]) in (alias | () -> true)) {
                return (valueof __cases__[n])(x);
            };
            n = n + 1;
        };
        raise __try_catch__.Err("No match found for " + alias);
    }
};

return {
    match_alias!,
}