TRUE := (x?, y?) -> x;
FALSE := (x?, y?) -> y;
AND := (x?, y?) -> x(y, @dynamic FALSE);
OR := (x?, y?) -> x(@dynamic TRUE, y);
NOT := (x?) -> x(@dynamic FALSE, @dynamic TRUE);
XOR := (x?, y?) -> x(@dynamic NOT(y), y);

to_bool := (f?) -> f(true, false);

// 自然数的丘奇编码
NUM := (n?) -> (f?, x?, n!) -> {
    y := wrap x;
    i := 0;
    while (i < n) {
        y = f(valueof y);
        i = i + 1;
    };
    return valueof y;
};

ADD := (m?, n?) -> (f?, x?, m!, n!) -> m(f, n(f, x));
MULT := (m?, n?) -> (f?, x?, m!, n!) -> m((g?, f!, n!) -> n(f, g), x);

SUCC := (n?) -> (f?, x?, n!) -> f(n(f, x));

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
CONS := (x?, y?) -> (f?, x!, y!) -> f(x, y);

PRED := (p?, CONS!, CAR!, CDR!, SUCC!) -> CONS(SUCC(CAR(p)), CAR(p));

@dynamic print(to_int(CDR(CONS(NUM(10), NUM(2)))));