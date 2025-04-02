(module
  (memory $mem 1)
  (func $add (param $lhs i32) (param $rhs i32) (result i32)
	(local $i i32)
	(loop $fillx
	      (i32.store
		(i32.add (i32.mul (local.get $i) (i32.const 4)) (i32.const 0))
		(local.get $lhs))
	      (local.set $i (i32.add (local.get $i) (i32.const 1)))
	      (i32.lt_s (local.get $i) (i32.const 4096))
	      br_if $fillx)
	(loop $filly
	      (i32.store
		(i32.add (i32.mul (local.get $i) (i32.const 4)) (i32.const 4096))
		(local.get $rhs))
	      (local.set $i (i32.add (local.get $i) (i32.const 1)))
	      (i32.lt_s (local.get $i) (i32.const 4096))
	      br_if $filly)
	(loop $sum
	      (i32.store
		(i32.add (i32.mul (local.get $i) (i32.const 4)) (i32.const 8192))
		(i32.add
		  (i32.load
		    (i32.add (i32.mul (local.get $i) (i32.const 4)) (i32.const 0)))
		  (i32.load
		    (i32.add (i32.mul (local.get $i) (i32.const 4)) (i32.const 4096)))))
	      (local.set $i (i32.add (local.get $i) (i32.const 1)))
	      (i32.lt_s (local.get $i) (i32.const 4096))
	      br_if $sum)
	(i32.load (i32.const 8192)))
  (export "add" (func $add)))
