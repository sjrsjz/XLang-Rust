try_catch := boundary (@dynamic (__stdlib_root!) -> dyn import (__stdlib_root + "/try_catch.xbc"))();
builtins := boundary (@dynamic (__stdlib_root!) -> dyn import (__stdlib_root + "/builtins.xbc"))();

match_alias := (cases?) -> &(builtins!, try_catch!) {
    return (x?) -> &(cases!, builtins => $this.builtins, try_catch => $this.try_catch) {
        n := 0;
        alias := aliasof x;
        while (n < $this.builtins.len($this.cases)) {
            if ((keyof $this.cases[n]) in (alias | () -> true)) {
                return (valueof $this.cases[n])(x);
            };
            n = n + 1;
        };

        n := 0;
        while (n < $this.builtins.len($this.cases)) {
            if ((keyof $this.cases[n]) == "_") {
                return (valueof $this.cases[n])(x);
            };
            n = n + 1;
        };
        raise $this.try_catch.Err("No match found for " + alias);
    }
};

match_value := (cases?) -> &(builtins!, try_catch!) {
    return (x?) -> &(cases!, builtins => $this.builtins, try_catch => $this.try_catch) {
        n := 0;
        while (n < $this.builtins.len($this.cases)) {
            if ((keyof $this.cases[n]) == x) {
                return (valueof $this.cases[n])(x);
            };
            n = n + 1;
        };

        n := 0;
        while (n < $this.builtins.len($this.cases)) {
            if (aliasof ($this.cases[n]) == ("default", )) {
                return (valueof $this.cases[n])(x);
            };
            n = n + 1;
        };
        raise $this.try_catch.Err("No match found for " + $this.builtins.string(x));
    }
};


return {
    match_alias!,
    match_value!,
}