TRUE := (x?, y?) -> x;
FALSE := (x?, y?) -> y;
AND := (x?, y?) -> x(y, FALSE);
OR := (x?, y?) -> x(TRUE, y);
NOT := (x?) -> x(FALSE, TRUE);
XOR := (x?, y?) -> x(NOT(y), y);

to_bool := (f?) -> f(true, false);

// 自然数的丘奇编码
NUM := (n?) -> (f?, x?, n => n) -> {
    y := wrap x;
    i := 0;
    while (i < n) {
        y = f(valueof y);
        i = i + 1;
    };
    return valueof y;
};

ADD := (m?, n?) -> (f?, x?, m => m, n => n) -> m(f, n(f, x));
MULT := (m?, n?) -> (f?, x?, m => m, n => n) -> m((g?, f => f) -> n(f, g), x);

SUCC := (n?) -> (f?, x?, n => n) -> f(n(f, x));

to_int := (f?) -> f((x?) -> x + 1, 0);

// print(NUM(10)(to_int, zero)());

// print(ADD(NUM(1),NUM(100))(to_int, zero)());


// print(to_bool(MULT(AND, XOR)(TRUE, TRUE)));
// print(to_bool(MULT(AND, XOR)(TRUE, FALSE)));
// print(to_bool(MULT(AND, XOR)(FALSE, TRUE)));
// print(to_bool(MULT(AND, XOR)(FALSE, FALSE)));

IF := (C?, T?, F?) -> C(T, F);

CAR := (p?, T => TRUE) -> p(T);
CDR := (p?, F => FALSE) -> p(F);
CONS := (x?, y?) -> (f?, x => x, y => y) -> f(x, y);

PRED := (p?, CONS => CONS, CAR => CAR, CDR => CDR, SUCC => SUCC) -> CONS(SUCC(CAR(p)), CAR(p));

print(
        CAR((x?) -> NUM(1)(CONS(NUM(1),NUM(0)), x))
);

print(to_int(CONS(NUM(10), NUM(2))(TRUE)));