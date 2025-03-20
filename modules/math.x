// XLang Mathematics Library - No power operator version

// Basic constants
PI := 3.14159265358979323846;
E := 2.71828182845904523536;
TAU := PI * 2;
SQRT2 := 1.41421356237309504880;
SQRT1_2 := 0.70710678118654752440;
LN2 := 0.69314718055994530942;
LN10 := 2.30258509299404568402;
LOG2E := 1.44269504088896340736;
LOG10E := 0.43429448190325182765;

// Create math library as a bindable object
math := bind {
    // Constants
    "PI" : PI,
    "E" : E,
    "TAU" : TAU,
    "SQRT2" : SQRT2,
    "SQRT1_2" : SQRT1_2,
    "LN2" : LN2,
    "LN10" : LN10,
    "LOG2E" : LOG2E,
    "LOG10E" : LOG10E,
    
    // Basic functions
    "abs" : (x => 0) -> {
        if (x < 0) { return -x; } else { return x; }
    },
    "sign" : (x => 0) -> {
        if (x > 0) { return 1; } else if (x < 0) { return -1; } else { return 0; }
    },
    "max" : (a => 0, b => 0) -> {
        if (a > b) { return a; } else { return b; }
    },
    "min" : (a => 0, b => 0) -> {
        if (a < b) { return a; } else { return b; }
    },
    "ceil" : (x => 0) -> {
        int_part := x - x % 1;
        if (x > int_part) { return int_part + 1; } else { return int_part; }
    },
    "floor" : (x => 0) -> {
        return x - x % 1;
    },
    "round" : (x => 0) -> {
        int_part := x - x % 1;
        frac := x % 1;
        if (frac >= 0.5) { return int_part + 1; } else { return int_part; }
    },
    "trunc" : (x => 0) -> {
        return x - x % 1;
    },
    
    // Power and exponential functions - implemented without power operator
    "exp" : (x => 0) -> {
        // Taylor series for e^x: 1 + x + x²/2! + x³/3! + ...
        if (x == 0) { return 1; };
        
        // Handle large positive values
        if (x > 20) { return 1000000000000000000000; }; // Approximate infinity
        
        // Handle large negative values
        if (x < -20) { return 0; }; // Approximate zero
        
        result := 1.0;
        term := 1.0;
        n := 1;
        
        // Calculate until term becomes very small
        while (n < 100) {
            term = term * x / n;
            result = result + term;
            
            if (term < 0.0000001 and (term > -0.0000001)) { break; };
            n = n + 1;
        };
        
        return result;
    },
    
    // Natural logarithm
    "log" : (x => 0, base => E) -> {
        if (x <= 0 or base <= 0 or base == 1) {
            return null; // Error case
        };
        
        // Natural logarithm approximation using Taylor series
        // ln(1+x) ≈ x - x²/2 + x³/3 - ... for |x| < 1
        if (base == E) {
            if (x == 1) { return 0; };
            
            // For values near 1, use Taylor series
            if (x > 0.5 and x < 1.5) {
                y := x - 1;
                result := 0.0;
                term := y;
                n := 1;
                
                while (n < 100) {
                    result = result + term / n;
                    term = -term * y;
                    
                    if (term / n < 0.0000001 and term / n > -0.0000001) { break; };
                    n = n + 1;
                };
                
                return result;
            };
            
            // For x > 1.5, use ln(ab) = ln(a) + ln(b)
            if (x > 1.5) {
                // Find k where x = 2^k * y and 0.5 < y < 1
                k := 0;
                y := x;
                
                while (y >= 2) {
                    y = y / 2;
                    k = k + 1;
                };
                
                return k * LN2 + self.log(y);
            };
            
            // For 0 < x < 0.5, use ln(1/x) = -ln(x)
            return -self.log(1 / x);
        };
        
        // Change of base formula: log_b(x) = ln(x) / ln(b)
        return self.log(x) / self.log(base);
    },
    
    // Power function without using power operator
    "pow" : (x => 0, y => 0) -> {
        // Special cases
        if (y == 0) { return 1; };
        if (x == 0) { return 0; };
        if (y == 1) { return x; };
        
        // Integer powers
        if (y == self.floor(y)) {
            result := 1.0;
            abs_y := self.abs(y);
            i := 0;
            
            while (i < abs_y) {
                result = result * x;
                i = i + 1;
            };
            
            if (y < 0) { return 1 / result; } else { return result; }
        };
        
        // For fractional powers, use e^(y*ln(x))
        if (x > 0) {
            return self.exp(y * self.log(x));
        };
        
        return null; // Error for negative base with fractional exponent
    },
    
    // Square root using Newton's method
    "sqrt" : (x => 0) -> {
        if (x < 0) { return null; }; // Error for negative values
        if (x == 0) { return 0; };
        if (x == 1) { return 1; };
        
        // Initial guess
        guess := x / 2;
        
        // Newton's method: x_{n+1} = (x_n + a/x_n)/2
        i := 0;
        while (i < 20) { // Limit iterations for practicality
            next_guess := (guess + x / guess) / 2;
            
            // Check for convergence
            if (self.abs(next_guess - guess) < 0.0000001) {
                return next_guess;
            };
            
            guess = next_guess;
            i = i + 1;
        };
        
        return guess;
    },
    
    // Cube root
    "cbrt" : (x => 0) -> {
        if (x == 0) { return 0; };
        
        sign := if (x < 0) -1 else 1;
        x = self.abs(x);
        
        // Initial guess
        guess := x / 3;
        
        // Newton's method for cube root: x_{n+1} = x_n - (x_n³ - a)/(3*x_n²)
        i := 0;
        while (i < 20) { // Limit iterations
            x2 := guess * guess;
            x3 := x2 * guess;
            next_guess := guess - (x3 - x) / (3 * x2);
            
            // Check for convergence
            if (self.abs(next_guess - guess) < 0.0000001) {
                return sign * next_guess;
            };
            
            guess = next_guess;
            i = i + 1;
        };
        
        return sign * guess;
    },
    
    "log10" : (x => 0) -> {
        return self.log(x, 10);
    },
    "log2" : (x => 0) -> {
        return self.log(x, 2);
    },
    
    // Trigonometric functions (using Taylor series approximations)
    "sin" : (x => 0) -> {
        // Normalize angle to [0, 2π)
        x = x % TAU;
        if (x < 0) { x = x + TAU; };
        
        // Taylor series approximation: sin(x) = x - x^3/3! + x^5/5! - ...
        result := 0.0;
        term := x;
        n := 1;
        sign := 1;
        
        while (n < 20) { // Limit iterations for practicality
            result = result + sign * term;
            
            // Calculate next term: term *= -x²/((2n)*(2n+1))
            term = term * x * x / (2 * n * (2 * n + 1));
            sign = -sign;
            n = n + 1;
        };
        
        return result;
    },
    
    "cos" : (x => 0) -> {
        // Normalize angle to [0, 2π)
        x = x % TAU;
        if (x < 0) { x = x + TAU; };
        
        // Taylor series approximation: cos(x) = 1 - x^2/2! + x^4/4! - ...
        result := 1.0;
        term := 1.0;
        n := 1;
        sign := -1;
        
        while (n < 20) { // Limit iterations for practicality
            // Calculate next term: term *= -x²/((2n-1)*(2n))
            term = term * x * x / ((2 * n - 1) * (2 * n));
            result = result + sign * term;
            sign = -sign;
            n = n + 1;
        };
        
        return result;
    },
    
    "tan" : (x => 0) -> {
        cos_x := self.cos(x);
        if (cos_x == 0) { return null; }; // Undefined
        return self.sin(x) / cos_x;
    },
    
    // Inverse trigonometric functions
    "asin" : (x => 0) -> {
        if (x < -1 or x > 1) { return null; }; // Domain error
        
        // Taylor series approximation for small x
        if (x > -0.5 and x < 0.5) {
            result := x;
            term := x;
            n := 0;
            
            while (n < 10) {
                // Calculate next term with recurrence relation
                term = term * x * x * (2 * n + 1) * (2 * n + 1) / ((2 * n + 2) * (2 * n + 3));
                result = result + term;
                n = n + 1;
            };
            
            return result;
        };
        
        // For larger values, use asin(x) = π/2 - asin(√(1-x²))
        if (x >= 0.5) {
            return PI / 2 - self.asin(self.sqrt(1 - x * x));
        };
        
        // For negative values, use asin(-x) = -asin(x)
        return -self.asin(-x);
    },
    
    "acos" : (x => 0) -> {
        if (x < -1 or x > 1) { return null; }; // Domain error
        return PI / 2 - self.asin(x);
    },
    
    "atan" : (x => 0) -> {
        // Taylor series for small x: atan(x) = x - x³/3 + x⁵/5 - ...
        if (x > -1 and x < 1) {
            result := x;
            term := x;
            n := 0;
            sign := -1;
            x_squared := x * x;
            
            while (n < 20) {
                term = term * x_squared;
                result = result + sign * term / (2 * n + 3);
                sign = -sign;
                n = n + 1;
            };
            
            return result;
        };
        
        // Use identity for larger values
        if (x >= 1) {
            return PI / 2 - self.atan(1 / x);
        };
        
        if (x <= -1) {
            return -PI / 2 - self.atan(1 / x);
        };
    },
    
    "atan2" : (y => 0, x => 0) -> {
        if (x == 0) {
            if (y > 0) { return PI / 2; };
            if (y < 0) { return -PI / 2; };
            return null; // Undefined at origin
        };
        
        if (x > 0) {
            return self.atan(y / x);
        };
        
        if (x < 0) {
            if (y >= 0) {
                return self.atan(y / x) + PI;
            } else {
                return self.atan(y / x) - PI;
            };
        };
    },
    
    // Hyperbolic functions
    "sinh" : (x => 0) -> {
        return (self.exp(x) - self.exp(-x)) / 2;
    },
    "cosh" : (x => 0) -> {
        return (self.exp(x) + self.exp(-x)) / 2;
    },
    "tanh" : (x => 0) -> {
        if (x > 20) { return 1; }; // Avoid overflow
        if (x < -20) { return -1; };
        
        exp_x := self.exp(x);
        exp_neg_x := self.exp(-x);
        return (exp_x - exp_neg_x) / (exp_x + exp_neg_x);
    },
    
    // Statistical functions
    "sum" : (arr => (,)) -> {
        total := 0;
        i := 0;
        while (i < len(arr)) {
            total = total + arr[i];
            i = i + 1;
        };
        return total;
    },
    
    "mean" : (arr => (,)) -> {
        if (len(arr) == 0) { return null; };
        return self.sum(arr) / len(arr);
    },
    
    "variance" : (arr => (,)) -> {
        if (len(arr) == 0) { return null; };
        mean := self.mean(arr);
        sum_squared_diff := 0;
        i := 0;
        while (i < len(arr)) {
            diff := arr[i] - mean;
            sum_squared_diff = sum_squared_diff + diff * diff;
            i = i + 1;
        };
        return sum_squared_diff / len(arr);
    },
    
    "std" : (arr => (,)) -> {
        return self.sqrt(self.variance(arr));
    },
    
    // Vector operations
    "vector_add" : (v1 => (,), v2 => (,)) -> {
        if (len(v1) != len(v2)) { return null; }; // Dimension mismatch
        result := (,);
        i := 0;
        while (i < len(v1)) {
            result = result + (v1[i] + v2[i],);
            i = i + 1;
        };
        return result;
    },
    
    "vector_sub" : (v1 => (,), v2 => (,)) -> {
        if (len(v1) != len(v2)) { return null; }; // Dimension mismatch
        result := (,);
        i := 0;
        while (i < len(v1)) {
            result = result + (v1[i] - v2[i],);
            i = i + 1;
        };
        return result;
    },
    
    "dot_product" : (v1 => (,), v2 => (,)) -> {
        if (len(v1) != len(v2)) { return null; }; // Dimension mismatch
        result := 0;
        i := 0;
        while (i < len(v1)) {
            result = result + v1[i] * v2[i];
            i = i + 1;
        };
        return result;
    },
    
    "vector_scale" : (v => (,), scalar => 0) -> {
        result := (,);
        i := 0;
        while (i < len(v)) {
            result = result + (v[i] * scalar,);
            i = i + 1;
        };
        return result;
    },
    
    "vector_magnitude" : (v => (,)) -> {
        sum_squared := 0;
        i := 0;
        while (i < len(v)) {
            sum_squared = sum_squared + v[i] * v[i];
            i = i + 1;
        };
        return self.sqrt(sum_squared);
    },
    
    "vector_normalize" : (v => (,)) -> {
        mag := self.vector_magnitude(v);
        if (mag == 0) { return null; }; // Cannot normalize zero vector
        return self.vector_scale(v, 1 / mag);
    },
    
    // Random number generation
    "random" : () -> {
        // Simple linear congruential generator
        // Using parameters from MMIX by Donald Knuth
        rnd_state := if (self.random_seed == null) 1 else self.random_seed;
        multiplier := 6364136223846793005;
        increment := 1442695040888963407;
        m := 1000000000; // Simpler modulus since XLang lacks 2^64
        
        rnd_state = (multiplier * rnd_state + increment) % m;
        self.random_seed = rnd_state;
        
        return rnd_state / m;
    },
    "random_seed" : null,
    "random_int" : (min => 0, max => 100) -> {
        return self.floor(self.random() * (max - min + 1)) + min;
    },
    
    // Utility functions
    "factorial" : (n => 0) -> {
        if (n < 0) { return null; }; // Undefined for negative numbers
        if (n == 0 or n == 1) { return 1; };
        
        result := 1;
        i := 2;
        while (i <= n) {
            result = result * i;
            i = i + 1;
        };
        return result;
    },
    
    "gcd" : (a => 0, b => 0) -> {
        a = self.abs(a);
        b = self.abs(b);
        
        while (b != 0) {
            temp := b;
            b = a % b;
            a = temp;
        };
        
        return a;
    },
    
    "lcm" : (a => 0, b => 0) -> {
        gcd := self.gcd(a, b);
        if (gcd == 0) { return 0; };
        return self.abs(a * b) / gcd;
    },
    
    "is_prime" : (n => 0) -> {
        if (n <= 1) { return false; };
        if (n <= 3) { return true; };
        if (n % 2 == 0 or n % 3 == 0) { return false; };
        
        i := 5;
        while (i * i <= n) {
            if (n % i == 0 or n % (i + 2) == 0) { return false; };
            i = i + 6;
        };
        
        return true;
    },
    
    "deg_to_rad" : (degrees => 0) -> {
        return degrees * PI / 180;
    },
    "rad_to_deg" : (radians => 0) -> {
        return radians * 180 / PI;
    },
};

// Example usage
print("Math library example usage:");
print("PI = " + math.PI);
print("sin(PI/2) = " + math.sin(math.PI/2));
print("cos(0) = " + math.cos(0));
print("sqrt(16) = " + math.sqrt(16));
print("log(100, 10) = " + math.log(100, 10));
print("Vector magnitude [3,4] = " + math.vector_magnitude((3,4)));