
use yajnir::jvm::{JavaVM, JniVersion, VmError};

fn main() -> Result<(), VmError> {

	let vmopts = JavaVM::default_args(JniVersion::new(20, 2)).expect("error retrieving default args");
	println!("{:?}", vmopts);

	Ok(())
}
