promise := (f?, then => (result?) -> {}, catch => (err?) -> {}) -> {
    wrapper := (f => f, then => then, catch => catch) -> {
        result := boundary f();
        if ("Err" in aliasof result) {
            return boundary catch(result);
        } else {
            return boundary then(result);
        };
    };
    return wrapper;
};

x := 0;

my_promise := promise(
    f => (x => x) -> {
        print("Simulating async operation...");
        if (x == 0) {
            raise Err::"Error occurred";
        };
        return x;
    },
    then => (result?) -> {
        print("Promise resolved with:", result);
    },
    catch => (err?) -> {
        print("Caught error:", err);
    }
);

async my_promise();
await my_promise;
print("Promise execution completed");