


// Taken from `jni` crate
macro_rules! java_vm_unchecked {
    ( $jvm:expr, $name:tt $(, $args:expr )* ) => ({
        log::trace!(concat!("calling unchecked JavaVM method: ", stringify!($name)));
		let jvm: JavaVM = $jvm;

        // SAFETY: JavaVM is always assumed to be a non-null, valid pointer to a JavaVM struct
        //         (unless the VM has been destroyed, in which case all bets are off and the
        //            JavaVM ptr should be considered dead)
        //         Also each function pointer is checked for null (Option as None) before use.
        //            If is null, then it returns an Err
        unsafe { java_vm_method!(jvm, $name)(jvm.ptr.as_ptr(), $($args),*) }
    })
}

// Taken from `jni` crate
macro_rules! java_vm_method {
    ( $jvm:expr, $name:tt ) => {{
        log::trace!(concat!("looking up JavaVM method ", stringify!($name)));
        let jvm: JavaVM = $jvm;

		// Note that JavaVM holds a non-null pointer, so no null-check needed until we lookup the function
        match (**jvm.ptr.as_ptr()).$name  {
            Some(meth) => {
                log::trace!(concat!("found JavaVM method ", stringify!($name)));
                meth
            }
            None => {
                log::trace!(concat!("JavaVM method ", stringify!($name), "not defined, returning error"));
                return Err(crate::jvm::VmError::MissingFunction(stringify!($name)));
            }
        }
    }};
}
