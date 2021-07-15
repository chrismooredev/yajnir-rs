use std::fmt;
use jtypes::InternalClassname;

// extern crate jni;
extern crate jni_sys;

#[cfg(test)] #[macro_use]
extern crate rusty_fork;

#[cfg(test)]
extern crate jvm_link;

use log;


#[macro_use] mod macros;
pub mod jvm;
mod env;
mod jref;

#[derive(Debug, PartialEq, Eq)]
struct NativeEscapeError {
	class: InternalClassname,
	method: String,
	overload_signature: Option<String>,
}

/// Generates the unmangled name the JVM searches for when looking for a given native method
///
/// See https://docs.oracle.com/en/java/javase/16/docs/specs/jni/design.html#resolving-native-method-names for details
fn _native_name(class: InternalClassname, method: &str, overload_signature: Option<&str>) -> Result<String, NativeEscapeError> {
	fn escape(s: &str) -> Option<String> {
		// arbitrary increase in capacity by 8
		let mut result = String::with_capacity(s.len() + 8);
		for c in s.chars() {
			match c {
				'/' => result.push('_'),
				'_' => result.push_str("_1"),
				';' => result.push_str("_2"),
				'[' => result.push_str("_3"),
				c if c.is_numeric() && result.chars().last().map(|ch| ch == '_').unwrap_or(false) => {
					// if the char is a number and will follow an underscore
					return None;
				},
				c if c.is_ascii_alphanumeric() => result.push(c),
				c => result.push_str(&format!("_0{:04x}", c as u16)),
			}
		}
		Some(result)
	}
	
	let (cls, meth, os) = (|| {
		let cls = escape(&*class)?;
		let meth = escape(method)?;
		let os = match overload_signature.map(|os| escape(os)) {
			Some(os) => Some(os?),
			None => None
		};
		
		Some((cls, meth, os))
	})().ok_or_else(|| NativeEscapeError {
		class,
		method: method.to_owned(),
		overload_signature: overload_signature.map(|s| s.to_owned()),
	})?;

	Ok(if let Some(os) = os {
		format!("Java_{}_{}__{}", cls, meth, os)
	} else {
		format!("Java_{}_{}", cls, meth)
	})
}

#[test]
fn native_names() {
	assert_eq!(Ok("Java_p_q_r_A_f"), _native_name(InternalClassname::new_unchecked("p/q/r/A"), "f", None).as_ref().map(|s| s.as_str()));
	assert_eq!(Ok("Java_p_q_r_A_f__ILjava_lang_String_2"), _native_name(InternalClassname::new_unchecked("p/q/r/A"), "f", Some("ILjava/lang/String;")).as_ref().map(|s| s.as_str()));
}

/// Translates a Rust bool to a Java boolean
pub(crate) fn r2j_bool(val: bool) -> jni_sys::jboolean {
	if val {
		jni_sys::JNI_TRUE
	} else {
		jni_sys::JNI_FALSE
	}
}
pub(crate) fn j2r_bool(val: jni_sys::jboolean) -> bool {
	if val == jni_sys::JNI_FALSE {
		false
	} else if val != jni_sys::JNI_TRUE {
		true
	} else {
		log::debug!("converting JNI bool {} to true, as it's non zero. Expected zero/one.", val);
		true
	}
}