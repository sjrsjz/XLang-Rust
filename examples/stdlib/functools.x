// wrap a function whose arguments are reset to their default values after each call
with_reset_params := (f?) -> {
    cloned := 0..(lengthof keyof f) |> (i?) -> {
        return copy (keyof f)[i];
    };
    wrapped := (...copy keyof f) -> &(default_args => cloned, f!) {
        f := $this.f;
        keyof f = ();
        result := f(...arguments);
        keyof this = 0..(lengthof $this.default_args) |> (i?, default_args => $this.default_args) -> {
            return copy default_args[i];
        };
        return result;
    };
    keyof this = (f?,);
    return wrapped;
};


identity := with_reset_params((v?) -> v);

compose := with_reset_params(
    (funcs => ()) -> {
        return (v?) -> {
            result := v;
            n := 0; while(n < lengthof(funcs)) {
                result = funcs[n](result);
                n = n + 1;
            };
            return result;
        };
    }
);

return {
    with_reset_params!,
    identity!,
    compose!,
}