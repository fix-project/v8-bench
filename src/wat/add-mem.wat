(module
  (memory $mem 1)
  (func $add (param $lhs i32) (param $rhs i32) (result i32)
  	(i32.store
	  (i32.const 0)
	  (local.get $lhs))
	(i32.store
	  (i32.const 4)
	  (local.get $rhs))
	(i32.store
	  (i32.const 8)
	  (i32.add
	    (i32.load (i32.const 0))
	    (i32.load (i32.const 4))))
	(i32.load (i32.const 8)))
  (export "add" (func $add)))
