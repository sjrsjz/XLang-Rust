try := (f?) -> bind {
    'result': wrap null,
    value => () -> return valueof self.result,
    catch => (err_handler?, f => f) -> {
        result := boundary f(...(keyof f));
        if ("Err" in aliasof result) {
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

x := 0;

result := try(
    (x => x) -> {
        if (x == 0) {
            raise Err::"Error occurred";
        };
        return x;
    }
).catch(
    (err?) -> {
        print("Caught error:", err);
    }
).finally(
    (finally?) -> {
        print("Finally block executed");
    }
).value();

print("Result:", result);


x := 1;

result := try(
    (x => x) -> {
        if (x == 0) {
            raise Err::"Error occurred";
        };
        return x;
    }
).catch(
    (err?) -> {
        print("Caught error:", err);
    }
).finally(
    (finally?) -> {
        print("Finally block executed");
    }
).value();

print("Result:", result);