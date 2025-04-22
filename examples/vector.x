mathlib := {
    clambda := @dynamic load_clambda("../modules/clambda_math_lib/clambda_math.so");
    {
        // 封装，由于 C 库一般不接受命名参数，所以这里包装一层
        sin => (x?) -> &clambda (sin::() -> dyn $this)(x),
        cos => (x?) -> &clambda (cos::() -> dyn $this)(x),
        tan => (x?) -> &clambda (tan::() -> dyn $this)(x),
        pow => (x?, y?) -> &clambda (pow::() -> dyn $this)(x, y),
        sqrt => (x?) -> &clambda (sqrt::() -> dyn $this)(x),
        round => (x?) -> &clambda (round::() -> dyn $this)(x),
        floor => (x?) -> &clambda (floor::() -> dyn $this)(x),
        ceil => (x?) -> &clambda (ceil::() -> dyn $this)(x),
        log => (x?) -> &clambda (log::() -> dyn $this)(x),
        log10 => (x?) -> &clambda (log10::() -> dyn $this)(x),
        exp => (x?) -> &clambda (exp::() -> dyn $this)(x),
        max => (x?) -> &clambda (max::() -> dyn $this)(x),
        min => (x?) -> &clambda (min::() -> dyn $this)(x),
        abs => (x?) -> &clambda (abs::() -> dyn $this)(x),
        pi => (pi::() -> dyn clambda)(),
        e => (e::() -> dyn clambda)(),
    }
};

print(mathlib.sqrt(4));

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
            str = str + @dynamic string(self.data[n]);
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