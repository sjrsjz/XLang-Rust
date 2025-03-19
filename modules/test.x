// create_async_func := () -> (n=>0) -> {
//     while (n = n + 1; n < 100000) {
//         yield n / 2;
//     };
//     return "success";
// };
// n:=0;
// asyncs := (,);
// while(n = n + 1; n <= 10) {
//     print("creating async function");
//     obj := create_async_func();
//     asyncs = asyncs + (obj,);
//     async obj();

// };
// print(asyncs);
// n:=0;
// while(n = n + 1; n < 1000000){
//     print(valueof asyncs[0])
// };



// obj1 := {
//     "A" : "This is A",
//     "B" : "This is B",
//     "C" : {
//         "D" : 1,
//         "E" : 2,
//     },
//     getB => () -> {
//         return self.B;
//     },
//     setB => (v => "") -> {
//         self.B = v;
//     },
// };

// print(obj1);

// obj2 := obj1;
// obj1.setB("This is obj1.B");

// print(obj2.getB());

// obj2 := copy obj1;
// obj2.B = "This is obj2.B";
// obj1.B = "This is obj1.B";

// print(obj2.getB());

extend := (obj => null, methods => (,)) -> {
    new_obj := (,);
    n := 0; while(n < len(obj)) {
        i := 0;
        found := while(i < len(methods)) {
            if (typeof obj[n] == "named") { if (keyof obj[n] == keyof methods[i]) { break true } };
            i = i + 1;
        };
        if (found != true) { new_obj = new_obj + (obj[n],) };
        n = n + 1;
    };
    n := 0; while(n < len(methods)) {
        new_obj = new_obj + (copy methods[n],);
        n = n + 1;
    };
    return new_obj;
};

obj1 := {
    "A" : "This is A",
    "B" : "This is B",
    "C" : {
        "D" : 1,
        "E" : 2,
    },
    getB => () -> {
        return self.B;
    },
    setB => (v => "") -> {
        self.B = v;
    },
};

extended_obj := extend(obj1, {
    "getA" => () -> {
        return self.A;
    },
    "setA" => (v => "") -> {
        self.A = v;
    },
});

print(extended_obj.getA());
print(extended_obj.getB());
extended_obj.setA("This is obj1.A");
extended_obj.setB("This is obj1.B");
print(extended_obj.getA());
print(extended_obj.getB());