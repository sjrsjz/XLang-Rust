retry := (f?, args => ()) -> (max_retry => 0, f => f, args => args, retry => 0) -> {
    while (retry < max_retry) {
        result := boundary f(...args);
        if ("Err" in aliasof result) {
            retry = retry + 1;
            continue;
        } else {
            return result;
        }
    };
    raise Err::"Max retry reached";
};

f := (x?) -> {
    if (x == 0) {
        raise Err::"Error occurred";
    };
    return x;
};

result := boundary retry(f, (0,))(3);
if ("Err" in aliasof result) {
    print("Error:", result);
} else {
    print("Result:", result);
};