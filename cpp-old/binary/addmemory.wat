(module
 (memory (;0;) 2)
 (func (export "add") (param $a i32) (param $b i32) (result i32)
  (i32.add (local.get $a) (local.get $b))))
