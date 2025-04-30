@required load_clambda;
@required io;
@required types;
mathlib := {
    clambda := load_clambda("../modules/clambda_math_lib/clambda_math.so");
    {
        sin => sin::() -> dyn clambda,
        cos => cos::() -> dyn clambda,
        tan => tan::() -> dyn clambda,
        pow => pow::() -> dyn clambda,
        sqrt => sqrt::() -> dyn clambda,
        round => round::() -> dyn clambda,
        floor => floor::() -> dyn clambda,
        ceil => ceil::() -> dyn clambda,
        log => log::() -> dyn clambda,
        log10 => log10::() -> dyn clambda,
        exp => exp::() -> dyn clambda,
        max => max::() -> dyn clambda,
        min => min::() -> dyn clambda,
        abs => abs::() -> dyn clambda,
        pi => (pi::() -> dyn clambda)(),
        e => (e::() -> dyn clambda)(),
    }
};
print := io.print;
string := types.string;
print(mathlib.sqrt(4,0));

Vector := (data => ()) -> &mathlib bind Vector::{
    constructor := this;
    'data' : data,
    '_mathlib' : $this,
    add => (v?) -> &constructor {
        if (lengthof self.data != lengthof v.data) {
            raise Err::"Dim Error"
        };
        added := 0..(lengthof self.data) |> (
            idx => 0, 
            A => self.data, 
            B => v.data
        ) -> A[idx] + B[idx];
        return $this(added)
    },
    sub => (v?) -> &constructor {
        if (lengthof self.data != lengthof v.data) {
            raise Err::"Dim Error"
        };
        subbed := 0..(lengthof self.data) |> (
            idx => 0, 
            A => self.data, 
            B => v.data
        ) -> A[idx] - B[idx];
        return $this(subbed)
    },
    dot => (v?) -> {
        if (lengthof self.data != lengthof v.data) {
            raise Err::"Dim Error"
        };
        itered := 0..(lengthof self.data) |> (
            idx => 0, 
            A => self.data, 
            B => v.data, 
            sum => 0.0
        ) -> (sum = sum + A[idx] * B[idx]);
        return itered[lengthof itered - 1]
    },
    norm => () -> {
        return self._mathlib.sqrt(self.dot(self));
    },
    scalar => (scalar?) -> &constructor {
        scaled := 0..(lengthof self.data) |> (
            idx => 0, 
            A => self.data,
            scalar!
        ) -> A[idx] * scalar;
        return $this(scaled)
    },
    normalize => () -> &constructor {
        normed := self.scalar(1 / self.norm());
        return normed
    },
    to_string => () -> {
        str := "[";
        n := 0;
        while (n < lengthof self.data) {
            str = str + string(self.data[n]);
            n = n + 1;
            if (n < lengthof self.data) {
                str = str + ", ";
            }
        };
        str = str + "]";
        return str
    },
};

A := Vector((1, 2, 3),);
B := Vector((4, 5, 6),);

C := A.add(B);
print(C.data);

D := A.sub(B);
print(D.data);

E := A.dot(B);
print(E);

F := A.norm();
print(F);

G := A.scalar(2);
print(G.data);

H := A.normalize();
print(H.data);

I := A.to_string();
print(I);