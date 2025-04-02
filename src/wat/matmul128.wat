(module
  (global $dim i32 (i32.const 128))
  (global $size (mut i32) (i32.const 0))
  (memory $mem 3) ;; >= ($dim**2)*4*3/65536

  (func $set (param $idx i32) (param $x i32) (param $y i32) (param $val i32)
	(local $base i32)
	(local $offset i32)
	(local.set $base (i32.mul (global.get $size) (local.get $idx)))
	(local.set $offset (i32.add (i32.mul (global.get $dim) (local.get $y)) (local.get $x)))
	(i32.store (i32.mul (i32.add (local.get $base) (local.get $offset)) (i32.const 4)) (local.get $val)))

  (func $get (param $idx i32) (param $x i32) (param $y i32) (result i32)
	(local $base i32)
	(local $offset i32)
	(local.set $base (i32.mul (global.get $size) (local.get $idx)))
	(local.set $offset (i32.add (i32.mul (global.get $dim) (local.get $y)) (local.get $x)))
	(i32.load (i32.mul (i32.add (local.get $base) (local.get $offset)) (i32.const 4))))

  (func $matmul (param $lhs i32) (param $rhs i32) (result i32)
	(local $x i32)
	(local $y i32)
	(local $i i32)
	(local $sum i32)

	(global.set $size (i32.mul (global.get $dim) (global.get $dim)))

	(local.set $y (i32.const 0))
	(loop $fill_0_outer
	      (local.set $x (i32.const 0))
	      (loop $fill_0_inner
		    (call $set (i32.const 0) (local.get $x) (local.get $y) (local.get $lhs))
		    (local.set $x (i32.add (local.get $x) (i32.const 1)))
		    (i32.lt_s (local.get $x) (global.get $dim))
		    br_if $fill_0_inner)
	      (local.set $y (i32.add (local.get $y) (i32.const 1)))
	      (i32.lt_s (local.get $y) (global.get $dim))
	      br_if $fill_0_outer)

	(local.set $y (i32.const 0))
	(loop $fill_1_outer
	      (local.set $x (i32.const 0))
	      (loop $fill_1_inner
		    (call $set (i32.const 1) (local.get $x) (local.get $y) (local.get $rhs))
		    (local.set $x (i32.add (local.get $x) (i32.const 1)))
		    (i32.lt_s (local.get $x) (global.get $dim))
		    br_if $fill_1_inner)
	      (local.set $y (i32.add (local.get $y) (i32.const 1)))
	      (i32.lt_s (local.get $y) (global.get $dim))
	      br_if $fill_1_outer)

	(local.set $y (i32.const 0))
	(loop $matmul_outer
	      (local.set $x (i32.const 0))
	      (loop $matmul_inner

		    (local.set $i (i32.const 0))
		    (local.set $sum (i32.const 0))
		    (loop $matmul_dot

			  (local.set $sum
				     (i32.add
				       (local.get $sum)
				       (i32.mul
					 (call $get (i32.const 0) (local.get $i) (local.get $y))
					 (call $get (i32.const 1) (local.get $x) (local.get $i)))))

			  (local.set $i (i32.add (local.get $i) (i32.const 1)))
			  (i32.lt_s (local.get $i) (global.get $dim))
			  br_if $matmul_dot)

		    (call $set (i32.const 2) (local.get $x) (local.get $y) (local.get $sum))

		    (local.set $x (i32.add (local.get $x) (i32.const 1)))
		    (i32.lt_s (local.get $x) (global.get $dim))
		    br_if $matmul_inner)
	      (local.set $y (i32.add (local.get $y) (i32.const 1)))
	      (i32.lt_s (local.get $y) (global.get $dim))
	      br_if $matmul_outer)

	(local.set $y (i32.const 0))
	(local.set $sum (i32.const 0))
	(loop $sum_outer
	      (local.set $x (i32.const 0))
	      (loop $sum_inner
		    (local.set $sum
			       (i32.add
				 (local.get $sum)
				 (call $get (i32.const 2) (local.get $x) (local.get $y))))
		    (local.set $x (i32.add (local.get $x) (i32.const 1)))
		    (i32.lt_s (local.get $x) (global.get $dim))
		    br_if $sum_inner)
	      (local.set $y (i32.add (local.get $y) (i32.const 1)))
	      (i32.lt_s (local.get $y) (global.get $dim))
	      br_if $sum_outer)
	(local.get $sum))

  (export "add" (func $matmul)))
