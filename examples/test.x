/* 一个非常操蛋的用来禁止缓存参数的内置函数的包装 */
builtins := bind {
    'builtin_print' : @dynamic print,
    'builtin_int' : @dynamic int,
    'builtin_float' : @dynamic float,
    'builtin_string' : @dynamic string,
    'builtin_bool' : @dynamic bool,
    'builtin_bytes' : @dynamic bytes,
    'builtin_input' : @dynamic input,
    print => () -> {
        result := self.builtin_print(...keyof this);
        keyof this = ();
        keyof self.builtin_print = ();
        return result;
    },
    int => () -> {
        result := self.builtin_int(...keyof this);
        keyof this = ();
        keyof self.builtin_int = ();
        return result;
    },
    float => () -> {
        result := self.builtin_float(...keyof this);
        keyof this = ();
        keyof self.builtin_float = ();
        return result;
    },
    string => () -> {
        result := self.builtin_string(...keyof this);
        keyof this = ();
        keyof self.builtin_string = ();
        return result;
    },
    bool => () -> {
        result := self.builtin_bool(...keyof this);
        keyof this = ();
        keyof self.builtin_bool = ();
        return result;
    },
    bytes => () -> {
        result := self.builtin_bytes(...keyof this);
        keyof this = ();
        keyof self.builtin_bytes = ();
        return result;
    },
    input => () -> {
        result := self.builtin_input(...keyof this);
        keyof this = ();
        keyof self.builtin_input = ();
        return result;
    }
};
print := builtins.print;
int := builtins.int;
float := builtins.float;
string := builtins.string;
bool := builtins.bool;
bytes := builtins.bytes;
input := builtins.input;

a:= bind T::{
    A => ()->null,
};
b:= deepcopy a;
print(aliasof a);
print(aliasof U::(wipe aliasof b));

in := "Hello";
check := (u?,v?)->if (u != v){raise Err::"Error Value"};
A := boundary {
    check(in,"hello");
    in
};

is_err := (v?) -> "Err" in {aliasof v | () -> true};

if (is_err(A)){
    print("ERROR:", A)
} else {
    print(A)
};

foo := (n=>0)->{a:=n;if(n<10){this(a+1)}};foo();

create_async_func := () -> (n=>0) -> {
    while (n = n + 1; n < 10) {
        emit n / 2;
    };
    return "success";
};
n:=0;
asyncs := (,);
while(n = n + 1; n <= 1) {
    print("creating async function");
    obj := create_async_func();
    asyncs = asyncs + (obj,);
    async obj();

};
print(asyncs);
n:=0;
while(n = n + 1; n < 1){
	i := 0;
	while(i = i + 1; i <= lengthof asyncs) {
		print(valueof asyncs[i - 1]);
	};
};
print("waiting for asyncs to finish");
n:=0;
while(n = n + 1; n <= lengthof asyncs) {
	await asyncs[n - 1];
};
print("all asyncs finished");

extend := (obj?, methods => (,)) -> {
    new_obj := (,);
    n := 0; while(n < @dynamic len(obj)) {
        i := 0;
        found := while(i < @dynamic len(methods)) {
            if (typeof obj[n] == "named") { if (keyof obj[n] == keyof methods[i]) { break true } };
            i = i + 1;
        };
        if (found != true) { new_obj = new_obj + (obj[n],) };
        n = n + 1;
    };
    n := 0; while(n < @dynamic len(methods)) {
        new_obj = new_obj + (methods[n],);
        n = n + 1;
    };
    return bind new_obj;
};
obj1 := bind {
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


classA := bind {
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

print(classA.getB());

// aliased := MyType::Type1::bind {
// 	print => () -> {
// 		print("This is a print function");
// 	},
// };

// aliased.print();
// print(aliasof aliased); // (Type1, MyType)

deepseek := bind {
	chat => () -> "服务器繁忙，请稍后重试",
};
print(deepseek.chat());

iter := (container?, wrapper?) -> 
	if (container == null or wrapper == null) {
		return () -> false;
	} else {
		return (container => container, wrapper => wrapper, n => 0) -> {
			if (n >= @dynamic len(container)) {
				return false;
			};
			wrapper = container[n];
			n = n + 1;
			return true;
		};
	};

arr := (1, 2, 3, 4, 5);
arr_iter := iter(arr, elem := wrap 0);
while(arr_iter()) {
	print(valueof elem);
};


RelationTable := (keys => (,)) -> {
    return bind {
        "keys": keys,
        "data": (,),
        append => (row?) -> {
            self.data = self.data + (row,);
        },
        key_idx => (keys => (,)) -> {
            idx := (,);
            n := 0; while(n < @dynamic len(keys)) {
                found := false;
                i := 0; while(i < @dynamic len(self.keys)) {
                    if (keys[n] == self.keys[i]) {
                        idx = idx + (i,);
                        found = true;
                        break;
                    };
                    i = i + 1;
                };
                if (found != true) {
                    return null;
                };
                n = n + 1;
            };
            return idx;
        },

        project => (keys => (,)) -> {
            idx := self.key_idx(keys);
            if (idx == null) {
                return null;
            };
            new_table := @dynamic RelationTable(keys);
            n := 0; while(n < @dynamic len(self.data)) {
                row := (,);
                i := 0; while(i < @dynamic len(idx)) {
                    row = row + (self.data[n][idx[i]],);
                    i = i + 1;
                };
                new_table.append(row);
                n = n + 1;
            };
            return new_table;
        },

        filter => (condition => (v?, table?) -> false) -> {
            new_table := @dynamic RelationTable(self.keys);
            n := 0; while(n < @dynamic len(self.data)) {
                if (condition(self.data[n], self) == true) {
                    new_table.append(self.data[n]);
                };
                n = n + 1;
            };
            return new_table;
        },
    }
};

table := RelationTable(("name", "age"),);
table.append(("Alice", 20),);
table.append(("Bob", 30),);
table.append(("Charlie", 40),);
table.append(("David", 50),);
table.append(("Eve", 60),);
print(table.data);
print(table.project(("name",),).data);

print(table.filter((row?, table?) -> {row[1] > 30}).project(("name",),));

fib := (n => 0) -> {
    if (n == 0) {
        return 0;
    } else if (n == 1) {
        return 1;
    } else {
        return this(n - 1) + this(n - 2);
    };
};
print(fib(4));

none := (n => 0) -> { 
	return 1;
	// if(n == 0) {
    //     return 0;
    // }
	// else {
	// 	return 1;//return none(0);
	// };
};
none2 := () -> @dynamic {
	none(1) + none(2);
};

n := 0;
while(n = n + 1; n < 10) {
	// j := 0;
	// while(j = j + 1; j < 100) {
	// 	if (n == j) {
	// 		break;
	// 	};
	// };
	none2();
};
