try_catch := boundary (@dynamic (__stdlib_root!) -> dyn import (__stdlib_root + "/try_catch.xbc"))();

promise := (f?, then => (result?) -> {}, catch => (err?) -> {}, try_catch!) -> {
    wrapper := (f => f, then => then, catch => catch, try_catch!) -> {
        result := boundary f();
        if (try_catch.is_err(result)) {
            return boundary catch(result);
        } else {
            return boundary then(result);
        };
    };
    return wrapper;
};

return {
    promise!,
};