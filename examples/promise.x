promise := (f?, then => (result?) -> {}, catch => (err?) -> {}) -> {
    wrapper := (f => f, then => then, catch => catch) -> {
        result := boundary f();
        if ("Err" in ((aliasof result) | () -> true)) {
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
        @dynamic print("Simulating async operation...");
        if (x == 0) {
            raise Err::"Error occurred";
        };
        return x;
    },
    then => (result?) -> {
        @dynamic print("Promise resolved with:", result);
    },
    catch => (err?) -> {
        @dynamic print("Caught error:", err);
    }
);

async my_promise();
await my_promise;
print("Promise execution completed");