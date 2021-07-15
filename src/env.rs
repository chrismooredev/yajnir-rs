
use std::marker::PhantomData;
use std::ptr::NonNull;
use jni_sys as js;

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct JniEnv<'a> {
	pub(crate) ptr: NonNull<jni_sys::JNIEnv>,
	pub(crate) _phantom: PhantomData<&'a ()>,
}

