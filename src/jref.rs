use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::Arc;

use crate::env::JniEnv;
use crate::jvm::JavaVM;


type RawJObject = NonNull<jni_sys::_jobject>;

// struct Timer;

// pub struct SomeJavaType<Ctx> {
// 	obj: NNjobject,
// 	ctx: Ctx,
// }
// impl<Ref: RichRef> SomeJavaType<Ref> {
// 	// instance fields & methods
// }
// impl<Ref: UpgradableRef> SomeJavaType<Ref> {
// 	// fn 
// }

// #[derive(Debug)] pub struct GlobalRef(NNJavaVM);
// #[derive(Debug)] pub struct LocalRef;
// #[derive(Debug)] pub struct AutoRef;

// #[derive(Debug)] pub struct GlobalObj<'a, Desc>(NNJniEnv<'a>, Rc<Desc>);
// #[derive(Debug)] pub struct LocalObj<'a, Desc>(NNJniEnv<'a>, Rc<Desc>);
// #[derive(Debug)] pub struct AutoObj<'a, Desc>(NNJniEnv<'a>, Rc<Desc>);

// pub trait RichRef {

// }
// trait UpgradableRef {
// 	type Upgraded;
// 	fn upgrade<'a, T>(kk env: NNJniEnv<'a>) -> Self::Upgraded
// }

/// A cacheable, thread-safe global reference to a non-null Java object.
#[derive(Debug)]
pub struct GlobalRef<T: RichJavaType> {
	jvm: JavaVM,
	obj: Arc<RawJObject>,
	desc: Arc<T::IDs>,
	_phantom: PhantomData<T>,
}

/// A local reference to a non-null Java object. Note that this type is especially suitable for type-safe method arguments for JNI native methods, when wrapped in Option.
#[derive(Debug)]
#[repr(transparent)]
pub struct LocalRef<T: RichJavaType> {
	obj: RawJObject,
	_phantom: PhantomData<*const T>,
}

/// A local, scoped reference to a non-null Java object. Note that this type is especially suitable for local variable references emitted from wrapper code.
#[derive(Debug)]
#[repr(transparent)]
pub struct AutoRef<T: RichJavaType> {
	obj: RawJObject,
	_phantom: PhantomData<*const T>,
}

#[derive(Debug)]
pub struct GlobalObj<'a, T: RichJavaType> {
	env: JniEnv<'a>,
	obj: Arc<RawJObject>,
	desc: Arc<T::IDs>,
	_phantom: PhantomData<&'a T>,
}

#[derive(Debug)]
pub struct LocalObj<'a, T: RichJavaType> {
	env: JniEnv<'a>,
	obj: RawJObject,
	desc: Arc<T::IDs>,
	_phantom: PhantomData<&'a T>,
}

#[derive(Debug)]
pub struct AutoObj<'a, T: RichJavaType> {
	env: JniEnv<'a>,
	obj: RawJObject,
	desc: Arc<T::IDs>,
	_phantom: PhantomData<&'a T>,
}

impl<T: RichJavaType> GlobalRef<T> {
	pub fn upgrade<'a>(&'_ self, env: &'a JniEnv<'a>) -> GlobalObj<'a, T> {
		GlobalObj {
			env: *env,
			obj: Arc::clone(&self.obj),
			desc: Arc::clone(&self.desc),
			_phantom: PhantomData,
		}
	}
}
impl<'a, T: RichJavaType> GlobalObj<'a, T> {
	pub fn downgrade(&self) -> GlobalRef<T> {
		// safety: passed pointer is not null
		todo!("downgrade globalobj to globalref");
		// let env: JNIEnv = unsafe { jni::JNIEnv::from_raw(self.env.ptr.as_ptr()) }.unwrap();
		// let jvm = env.get_java_vm().expect("GetJavaVM failure");
		// let nnjvm = NNJavaVM {
		// 	ptr: NonNull::new(jvm.get_java_vm_pointer()).expect("JNIEnv used null JavaVM")
		// };

		// GlobalRef {
		// 	jvm: nnjvm,
		// 	obj_desc: Arc::clone(&self.obj_desc),
		// 	_phantom: PhantomData,
		// }
	}
}

// impl_upgrade!(GlobalRef, GlobalObj);
// impl_upgrade!(LocalRef, LocalObj);
// impl_upgrade!(AutoRef, AutoObj);
// impl<'a, T: RichJavaType> GlobalObj<'a, T> {
// 	pub fn downgrade(&self) -> GlobalRef<T> {
// 		GlobalRef {
// 			jvm,
// 			obj: self.obj,
// 			desc: self.desc,
// 			_phantom: PhantomData,
// 		}
// 	}
// }
// impl<'a, T: RichJavaType> LocalObj<'a, T> {
// 	pub fn downgrade(&self) -> LocalRef<T> {
// 		LocalRef {
			
// 		}
// 	}
// }
// impl<'a, T: RichJavaType> AutoRef<'a, T> {
// 	pub fn downgrade(&self) -> AutoRef<T> {
// 		AutoRef {
			
// 		}
// 	}
// }


pub trait RichJavaType {
	// Descriptor object should contain a GlobalRef to a class, as well as method/field IDs
	// all of these should be thread/invocation safe, so no specific lifetime requirements
	// as long as it has been created everything should stay valid
	// (unless the JVM shuts down - then all bets are off)
	type IDs;

	fn descriptors<'thread>(env: JniEnv<'thread>) -> Arc<Self::IDs>;
}
