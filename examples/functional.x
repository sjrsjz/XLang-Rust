// XLang 纯函数式编程库

// 创建函数式编程库作为可绑定对象
fp := bind {
    // 高阶函数
    map => (arr => (), fn => (x?) -> null) -> {
        result := ();
        i := 0;
        while (i < @dynamic lengthof(arr)) {
            result = result + (fn(arr[i]),);
            i = i + 1;
        };
        return result;
    },
    
    filter => (arr => (), predicate => (x?) -> false) -> {
        result := ();
        i := 0;
        while (i < @dynamic lengthof(arr)) {
            if (predicate(arr[i])) {
                result = result + (arr[i],);
            };
            i = i + 1;
        };
        return result;
    },
    
    reduce => (arr => (), fn => (acc?, x?) -> null, initial?) -> {
        if (@dynamic lengthof(arr) == 0) { return initial; };
        
        acc := copy initial;
        i := 0;
        while (i < @dynamic lengthof(arr)) {
            acc = fn(acc, arr[i]);
            i = i + 1;
        };
        return acc;
    },
    
    // 函数组合
    compose => (f => (x?) -> null, g => (x?) -> null) -> {
        return (x?, f!, g!) -> {
            return f(g(x));
        };
    },
    
    pipe => (x?, fns => ()) -> {
        result := copy x;
        i := 0;
        while (i < @dynamic lengthof(fns)) {
            result = fns[i](result);
            i = i + 1;
        };
        return result;
    },
    
    
    partial => (fn => (x?) -> null, args => ()) -> {
        return (x?, args!, fn!) -> {
            all_args := copy args + (x,);
            return fn(all_args);
        };
    },
    
    // Option 类型模拟
    Option => bind {
        Some => (value?) -> {
            return Option::Some::bind {
                "value": value,
                "Some": self.Some,
                is_some => () -> { return true; },
                is_none => () -> { return false; },
                unwrap => () -> { return self.value; },
                map => (fn => (x?) -> null) -> {
                    return self.Some(fn(self.value));
                },
                and_then => (fn => (x?) -> null) -> {
                    return fn(self.value);
                },
                or_else => (fn => () -> null) -> {
                    return self.Some(self.value);
                },
                unwrap_or => (default?) -> {
                    return self.value;
                },
                unwrap_or_else => (fn => () -> null) -> {
                    return self.value;
                }
            };
        },
        
        None => () -> {
            return Option::None::bind {
                "type": "None",
                "None": self.None,
                is_some => () -> { return false; },
                is_none => () -> { return true; },
                unwrap => () -> { return null; },
                map => (fn => (x?) -> null) -> {
                    return self.None();
                },
                and_then => (fn => (x?) -> null) -> {
                    return self.None();
                },
                or_else => (fn => () -> null) -> {
                    return fn();
                },
                unwrap_or => (default?) -> {
                    return default;
                },
                unwrap_or_else => (fn => () -> null) -> {
                    return fn();
                }
            };
        }
    },
    
    // Result 类型模拟
    Result => bind {
        Ok => (value?) -> {
            return Result::Ok::bind {
                "type": "Ok",
                "value": value,
                "Ok": self.Ok,
                is_ok => () -> { return true; },
                is_err => () -> { return false; },
                unwrap => () -> { return self.value; },
                unwrap_err => () -> { return null; }, // 错误情况
                map => (fn => (x?) -> null) -> {
                    return self.Ok(fn(self.value));
                },
                map_err => (fn => (x?) -> null) -> {
                    return self.Ok(self.value);
                },
                and_then => (fn => (x?) -> null) -> {
                    return fn(self.value);
                },
                or_else => (fn => (x?) -> null) -> {
                    return self.Ok(self.value);
                }
            };
        },
        
        Err => (error?) -> {
            return Result::Err::bind {
                "type": "Err",
                "error": error,
                "Err": self.Err,
                is_ok => () -> { return false; },
                is_err => () -> { return true; },
                unwrap => () -> { return null; }, // 错误情况
                unwrap_err => () -> { return self.error; },
                map => (fn => (x?) -> null) -> {
                    return self.Err(self.error);
                },
                map_err => (fn => (x?) -> null) -> {
                    return self.Err(fn(self.error));
                },
                and_then => (fn => (x?) -> null) -> {
                    return self.Err(self.error);
                },
                or_else => (fn => (x?) -> null) -> {
                    return fn(self.error);
                }
            };
        }
    },
    
    // 通用工具函数
    identity => (x?) -> {
        return x;
    },
    
    constant => (x?) -> {
        return (x!) -> {
            return x;
        };
    },
    
    // 不可变数据操作
    array => {
        append => (arr => (), value?) -> {
            return arr + (value,);
        },
        
        prepend => (arr => (), value?) -> {
            return (value,) + arr;
        },
        
        concat => (arr1 => (), arr2 => ()) -> {
            return arr1 + arr2;
        },
        
        take => (arr => (), n => 0) -> {
            result := ();
            i := 0;
            count := if (n > @dynamic lengthof(arr)) (@dynamic lengthof(arr)) else n;
            
            while (i < count) {
                result = result + (arr[i],);
                i = i + 1;
            };
            
            return result;
        },
        
        drop => (arr => (), n => 0) -> {
            result := ();
            i := n;
            
            while (i < @dynamic lengthof(arr)) {
                result = result + (arr[i],);
                i = i + 1;
            };
            
            return result;
        },
        
        find => (arr => (), predicate => (x?) -> false) -> {
            i := 0;
            while (i < @dynamic lengthof(arr)) {
                if (predicate(arr[i])) {
                    return self.Option.Some(arr[i]);
                };
                i = i + 1;
            };
            return self.Option.None();
        },
        
        all => (arr => (), predicate => (x?) -> false) -> {
            i := 0;
            while (i < @dynamic lengthof(arr)) {
                if (not predicate(arr[i])) {
                    return false;
                };
                i = i + 1;
            };
            return true;
        },
        
        any => (arr => (), predicate => (x?) -> false) -> {
            i := 0;
            while (i < @dynamic lengthof(arr)) {
                if (predicate(arr[i])) {
                    return true;
                };
                i = i + 1;
            };
            return false;
        },
        
        zip => (arr1 => (), arr2 => ()) -> {
            result := ();
            i := 0;
            len := if (@dynamic lengthof(arr1) < @dynamic lengthof(arr2)) (@dynamic lengthof(arr1)) else @dynamic lengthof(arr2);
            
            while (i < len) {
                result = result + ((arr1[i], arr2[i]),);
                i = i + 1;
            };
            
            return result;
        },
        
        unzip => (pairs => ()) -> {
            fst := ();
            snd := ();
            i := 0;
            
            while (i < @dynamic lengthof(pairs)) {
                if (@dynamic lengthof(pairs[i]) >= 2) {
                    fst = fst + (pairs[i][0],);
                    snd = snd + (pairs[i][1],);
                };
                i = i + 1;
            };
            
            return (fst, snd);
        }
    },
    
    // 函数复合工具
    flip => (fn => (a?, b?) -> null) -> {
        return (b?, a?, fn!) -> {
            return fn(a, b);
        };
    },
    
    // 记忆化（memoization）
    memoize => (fn => (x?) -> null) -> {
        cache := ();
        
        return (x?, cache!, fn!) -> {
            key := @dynamic string(x);
            if (key in cache) {
                return cache[key];
            };
            
            result := fn(x);
            cache[key] = result;
            return result;
        };
    },
    iter => (container?, wrapper?) -> 
        if (container == null or wrapper == null) {
            return () -> false;
        } else {
            return (container => container, wrapper => wrapper, n => 0) -> {
                if (n >= @dynamic lengthof(container)) {
                    return false;
                };
                wrapper = container[n];
                n = n + 1;
                return true;
            };
        },
    Iterator => (container?) -> {
        return Iterator::bind {
            "container": container,
            "index": 0,
            next => () -> {
                if (self.index >= @dynamic lengthof(self.container)) {
                    return null;
                };
                value := self.container[self.index];
                self.index = self.index + 1;
                return value;
            },
            has_next => () -> {
                return self.index < @dynamic lengthof(self.container);
            },
            reset => () -> {
                self.index = 0;
            },
            step => (step => 1) -> {
                self.index = self.index + step;
                return self.index < @dynamic lengthof(self.container);
            },
            get => () -> {
                if (self.index >= @dynamic lengthof(self.container)) {
                    return null;
                };
                return self.container[self.index];
            },
        };
    },
    extend => (obj?, methods => ()) -> {
        new_obj := ();
        n := 0; while(n < @dynamic lengthof(obj)) {
            i := 0;
            found := while(i < @dynamic lengthof(methods)) {
                if (typeof obj[n] == "named") { if (keyof obj[n] == keyof methods[i]) { break true } };
                i = i + 1;
            };
            if (found != true) { new_obj = new_obj + (obj[n],) };
            n = n + 1;
        };
        n := 0; while(n < @dynamic lengthof(methods)) {
            new_obj = new_obj + (methods[n],);
            n = n + 1;
        };
        return bind new_obj;
    }
};

return fp;