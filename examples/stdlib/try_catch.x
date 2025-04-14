Err := (v?) -> bind Err::{
    'result' : v,
    value => () -> self.result,
};
Ok := (v?) -> bind Ok::{
    'result' : v,
    value => () -> self.result,
};
is_err := (v?) -> "Err" in {aliasof v | () -> true};

try := (f?, is_err!) -> bind {
    'result': wrap null,
    value => () -> return valueof self.result,
    catch => (err_handler?, f!, is_err!) -> {
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

return {
    try!,
    is_err!,
    Err!,
    Ok!,
}