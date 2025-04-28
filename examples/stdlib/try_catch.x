Err := (v?) -> bind Err::{
    'result' : v,
    value => () -> self.result,
};
Ok := (v?) -> bind Ok::{
    'result' : v,
    value => () -> self.result,
};
is_err := (v?) -> "Err" in {aliasof v | () -> true};

try := (f?) -> bind {
    'result': wrap null,
    value => () -> return valueof self.result,
    catch => (err_handler?) -> {
        result := boundary f(...(keyof f));
        if (is_err(result)) {
            err_handler(result);
        } else {
            self.result = result;
        };
        return self;
    },
    finally => (finally_handler?) -> {
        result := boundary finally_handler(...(keyof finally_handler));
        return self;
    },
};

try_catch := (pair?) -> {
    return (valueof pair)(keyof pair, boundary {
        return Ok((keyof pair)());
    });
};

return {
    try!,
    is_err!,
    Err!,
    Ok!,
    try_catch!,
}