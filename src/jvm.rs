use std::ffi::CStr;
use std::fmt;
use std::borrow::Cow;
use std::convert::TryInto;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::os::raw::c_char;
use std::ptr::NonNull;

use log;
use jni_sys as js;

use crate::env::JniEnv;
use crate::j2r_bool;
use crate::r2j_bool;


// JNI_OnLoad
// JNI_OnUnload

// jni_sys::

/// JNI
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct JniVersion {
	pub major: u16,
	pub minor: u16,
}
impl JniVersion {
	pub const V1_1: JniVersion = JniVersion::new(1, 1);
	pub const V1_2: JniVersion = JniVersion::new(1, 2);
	pub const V1_4: JniVersion = JniVersion::new(1, 4);
	pub const V1_6: JniVersion = JniVersion::new(1, 6);
	pub const V1_8: JniVersion = JniVersion::new(1, 8);
	pub const V9: JniVersion = JniVersion::new(9, 0);
	pub const V10: JniVersion = JniVersion::new(10, 0);

	pub const fn new(major: u16, minor: u16) -> JniVersion {
		JniVersion { major, minor }
	}

	const fn as_native(&self) -> u32 {
		((self.major as u32) << 16) | (self.minor as u32)
	}

	const fn from_native(n: u32) -> JniVersion {
		JniVersion {
			major: (n & 0xFFFF0000 >> 16) as u16,
			minor: (n & 0x0000FFFF >>  0) as u16,
		}
	}
}
impl fmt::Display for JniVersion {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}.{}", self.major, self.minor)
	}
}

/// A threadsafe pointer to an existing (but not necessarily active) Java VM
#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct JavaVM {
	pub(crate) ptr: NonNull<jni_sys::JavaVM>,
}

impl JavaVM {
	pub fn default_args(target_version: JniVersion) -> Result<VmOptions, VmError> {
		// TODO: Does this have an actual failure state? (other than bad params)
		
		let empty_option = js::JavaVMOption {
			optionString: std::ptr::null_mut(),
			extraInfo: std::ptr::null_mut(),
		};
		let n_options = 64;
		let mut options: Vec<js::JavaVMOption> = vec![empty_option; n_options];

		// must set vm_args->version field
		let mut args: js::JavaVMInitArgs = js::JavaVMInitArgs {
			version: target_version.as_native() as i32,
			nOptions: n_options as i32,
			options: options.as_mut_ptr(),
			ignoreUnrecognized: r2j_bool(true),
		};
		let res = VmError::assert_ok(unsafe { jni_sys::JNI_GetDefaultJavaVMInitArgs(&mut args as *mut js::JavaVMInitArgs as *mut c_void) })?;
		assert_eq!(res, 0, "JNI_GetDefaultJavaVMInitArgs did not return an error constant or JNI_OK as expected (returned {})", res);
		assert!(! args.options.is_null(), "JNI_GetDefaultJavaVMInitArgs returned null pointer for args.options with successful call");
		assert!(  args.nOptions <= n_options as i32, "JVM had more than {} default arguments (provided {}). Please raise the limit in yajnir.", n_options, args.nOptions);

		eprintln!("processing result (nOptions = {})", args.nOptions);

		// JNI docs to not specifify if the options are statically owned/how long they will last, so make our own copies
		let opts: Vec<String> = options[..args.nOptions as usize].iter()
			.filter(|opt_ent| ! opt_ent.optionString.is_null())
			.map(|opt_ent| unsafe { CStr::from_ptr(opt_ent.optionString) })
			.map(|cs| cesu8::from_java_cesu8(cs.to_bytes()).map(|c| String::from(c)))
			.collect::<Result<Vec<String>, cesu8::Cesu8DecodingError>>()?;

		eprintln!("done processing result");

		Ok(VmOptions {
			version: JniVersion::from_native(args.version as u32),
			options: opts.into_iter().map(|s| Cow::Owned(s)).collect(),
			ignore_unrecognized: j2r_bool(args.ignoreUnrecognized),
		})
	}

	/// Return the created JavaVMs that exist in this process.
	/// 
	/// Note that most or all JVMs only support one instance per process.
	pub fn created_jvms() -> Result<Vec<JavaVM>, VmError> {
		let mut buf: Vec<*mut jni_sys::JavaVM> = vec![std::ptr::null_mut(); 1];
		let mut true_len: js::jsize = 0;

		// loop around until we have the right number
		loop {
			let res = VmError::assert_ok(unsafe {
				jni_sys::JNI_GetCreatedJavaVMs(
					buf.as_mut_ptr(),
					buf.len().try_into().expect("more JavaVMs exist than positive integers?"),
					&mut true_len as *mut js::jsize
				)
			})?;
			assert_eq!(res, 0, "JNI_GetCreatedJavaVMs did not return an error constant or JNI_OK as expected (returned {})", res);

			let buf_len = buf.len().try_into().expect("more JavaVMs exist than positive integers?");
			if true_len > buf_len {
				buf.resize(true_len as usize, std::ptr::null_mut());
				continue;
			} else if true_len < buf_len {
				buf.truncate(true_len as usize);
			}

			// true_len == buf_len
			let jvms = buf.into_iter()
				.map(|pjvm| NonNull::new(pjvm).expect("GetCreatedJavaVMs returned null pointers for a JavaVM"))
				.map(|nnjvm| JavaVM { ptr: nnjvm })
				.collect();

			return Ok(jvms);
		}
	}

	/// Creates a Java Virtual Machine using the specified options.
	/// 
	/// The current thread will be attached, and become the main thread.
	/// 
	/// Creating multiple VMs in a single process is not supported.
	/// 
	/// This implementation does not support 'vfprintf', 'exit', or 'abort' options, and will panic if they are provided.
	///
	/// ```
	/// use yajnir::jvm::{JavaVM, JniVersion, VmOptions, VmError};
	/// # fn main() -> Result<(), VmError> {
	/// 
	/// let options = VmOptions::new(JniVersion::V10);
	/// let (vm, env) = JavaVM::create(options)?;
	/// /* actions that require a JavaVM or JniEnv */
	/// vm.destroy()?;
	/// # Ok(())
	/// # }
	/// ```
	///
	pub fn create<'env>(opts: VmOptions) -> Result<(JavaVM, JniEnv<'env>), VmError> {
		
		// would love to support these, but I couldn't find any documentation on them
		if opts.options.iter().any(|s| s == "vfprintf") {
			panic!("tried to use `vfprintf` option when starting jvm");
		}
		if opts.options.iter().any(|s| s == "exit") {
			panic!("tried to use `exit` option when starting jvm");
		}
		if opts.options.iter().any(|s| s == "abort") {
			panic!("tried to use `abort` option when starting jvm");
		}

		let mut vmoptstrs: Vec<Cow<[u8]>> = opts.options.iter()
			.map(|s| cesu8::to_java_cesu8(s))
			.collect();

		let mut vmopts: Vec<js::JavaVMOption> = vmoptstrs.iter()
			.map(|b| {
				js::JavaVMOption {
					optionString: b.as_ptr() as *mut c_char,
					extraInfo: std::ptr::null_mut(),
				}
			})
			.collect();

		let mut init_args: js::JavaVMInitArgs = js::JavaVMInitArgs {
			version: opts.version.as_native() as i32,
			nOptions: opts.options.len() as i32,
			options: vmopts.as_mut_ptr(),
			ignoreUnrecognized: r2j_bool(opts.ignore_unrecognized),
		};

		let mut raw_jvm_ptr: *mut js::JavaVM = std::ptr::null_mut();
		let mut raw_jenv_ptr: *mut js::JNIEnv = std::ptr::null_mut();
		let res = VmError::assert_ok(unsafe {
			js::JNI_CreateJavaVM(
				&mut raw_jvm_ptr as *mut *mut js::JavaVM,
				&mut raw_jenv_ptr as *mut *mut js::JNIEnv as *mut *mut c_void,
				&mut init_args as *mut js::JavaVMInitArgs as *mut c_void
			)
		})?;
		assert_eq!(res, 0, "JNI_GetCreatedJavaVMs did not return an error constant or JNI_OK as expected (returned {})", res);

		let jvm = NonNull::new(raw_jvm_ptr).expect("JNI_CreateJavaVM output null pointer for JavaVM without returning error");
		let jenv = NonNull::new(raw_jenv_ptr).expect("JNI_CreateJavaVM output null pointer for JNIEnv without returning error");

		// TODO: would it be better to simply return the JavaVM and let the user retrieve the JniEnv seperately?
		//       this would better enforce the lifetime requirement of JniEnv being a part of the JavaVM

		Ok((
			JavaVM { ptr: jvm },
			JniEnv { ptr: jenv, _phantom: PhantomData }
		))
	}

	pub fn destroy(self) -> Result<(), VmError> {
		// TODO: assert that no exception is pending? Clear it if it is?

		let res = VmError::assert_ok(java_vm_unchecked!(self, DestroyJavaVM))?;
		assert_eq!(res, 0, "JavaVM.DestroyJavaVM did not return an error constant or JNI_OK as expected (returned {})", res);

		Ok(())
	}

	pub fn create_with<O, F: Fn(JavaVM, JniEnv) -> O>(opts: VmOptions, func: F) -> Result<O, (VmError, Option<O>)> {
		// three failure conditions
		// create
		//   user (who may or may not destroy jvm)
		// destroy (if fails, how to return user rtn?)
		
		let (jvm, jenv) = JavaVM::create(opts)
			.map_err(|e| (e, None))?;
		let res = func(jvm, jenv);
		match jvm.destroy() {
			Ok(()) => Ok(res),
			Err(e) => return Err((e, Some(res))),
		}
	}
}

#[derive(Debug, Clone)]
pub struct VmOptions {
	version: JniVersion,
	options: Vec<Cow<'static, str>>,
	ignore_unrecognized: bool,
}
impl VmOptions {
	/// Creates a basic VmOptions struct, which passes an empty list of arguments to the JVM upon creation while checking the version number.
	pub fn new(version: JniVersion) -> VmOptions {
		VmOptions {
			version,
			options: Vec::new(),
			ignore_unrecognized: false,
		}
	}

	/// Creates a basic VmOptions struct, which passes the provided list of arguments to the JVM upon creation while checking the version number.
	///
	/// If any passed arguments passed to the JVM are unrecognized, the VM will error on creation.
	pub fn with_opts(version: JniVersion, opts: Vec<Cow<'static, str>>) -> VmOptions {
		VmOptions {
			version,
			options: opts,
			ignore_unrecognized: false,
		}
	}

	/// Creates a basic VmOptions struct, which passes the provided list of arguments to the JVM upon creation while checking the version number.
	///
	/// If any passed arguments passed to the JVM are unrecognized, the VM will ignore them on creation.
	pub fn with_unrecognized_opts(version: JniVersion, opts: Vec<Cow<'static, str>>) -> VmOptions {
		VmOptions {
			version,
			options: opts,
			ignore_unrecognized: true,
		}
	}

	pub fn replace_options(&mut self, opts: Vec<Cow<'static, str>>) {
		self.options = opts;
	}
	pub fn options(&self) -> &[Cow<'static, str>] {
		&self.options
	}
	pub fn allow_unrecognized_options(&mut self, allow: bool) {
		self.ignore_unrecognized = allow;
	}

	/// Pushes a system property argument onto the VM's arguments list.
	pub fn push_property(&mut self, name: &str, value: &str) {
		self.options.push(Cow::from(format!("-D{}={}", name, value)));
	}
}



#[derive(Debug, thiserror::Error)]
pub enum VmError {
	#[error("an unknown error has occured")]
	Unknown,
	#[error("thread of JNI function caller not attached to a JVM")]
	Detached,
	#[error("incompatible JNI version requested")]
	BadVersion,
	#[error("not enough memory")]
	NotEnoughMemory,
	#[error("JVM already exists on this thread")]
	VMExists(Result<JavaVM, Box<VmError>>),
	#[error("invalid arguments were passed to a JNI function")]
	InvalidArguments,

	#[error("attempt to use missing JavaVM.{} function (incompatible JNI/JVM version?)", .0)]
	MissingFunction(&'static str),

	#[error("a JavaVM function returned a malformed CESU8 string")]
	BadCesu8String(#[from] cesu8::Cesu8DecodingError),
}
impl VmError {
	/// Checks that a given number (likely from the result of a JNI function) does not correspond to an error constant.
	///
	/// If it does, than Err(VmError) is returned. If the code does not correspond, then it is passed through as an Ok(_)
	///
	/// Note that this function does not check for the result to be JNI_OK or positive, just that it is not an error.
	pub fn assert_ok(func_result: js::jint) -> Result<js::jint, VmError> {
		match func_result {
			js::JNI_ERR => Err(VmError::Unknown),
			js::JNI_EDETACHED => Err(VmError::Detached),
			js::JNI_EVERSION => Err(VmError::BadVersion),
			js::JNI_ENOMEM => Err(VmError::NotEnoughMemory),
			js::JNI_EEXIST => {
				let jvm = JavaVM::created_jvms()
					.map_err(|e| Box::new(e))
					.map(|lis| *lis.get(0).expect("JavaVM to exist if JNI_EEXIST was returned"));

				Err(VmError::VMExists(jvm))
			},
			js::JNI_EINVAL => Err(VmError::InvalidArguments),
			fr => Ok(fr),
		}
	}
}



#[cfg(test)]
mod tests {
	use crate::jvm::{JavaVM, JniVersion, VmOptions, VmError};

	rusty_fork::rusty_fork_test! {
		#[test]
		fn create_destroy_jvm() {
			// return Ok(());
			let options = VmOptions::new(JniVersion::V10);
			let (vm, _env) = JavaVM::create(options).expect("error creating vm");

			/* actions that require a JavaVM or JniEnv */

			vm.destroy().expect("error destroying vm");
		}

		#[test]
		fn destroy_jvm_twice() {
			let options = VmOptions::new(JniVersion::V10);
			let (vm, _env) = JavaVM::create(options).expect("error creating vm");

			/* actions that require a JavaVM or JniEnv */

			vm.destroy().expect("error destroying vm (first)");
			vm.destroy().expect_err("error destroying vm (second)");
		}

	}
}
