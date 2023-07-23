// #![allow(unused)]

mod impl_match;
mod gen;

#[test]
fn test_main() {

    gen::state::from_book::first::main();
    gen::state::from_book::without_return::main();
    gen::state::from_book::with_arg_ref_mut::main();
    gen::state::from_book::task_and_str_and_result::main();
    gen::state::from_book::str_and_result::main();
    gen::state::from_book::move_self_2_impl::main();
    gen::state::from_book::two_result::main();

    gen::state::state_machine::test();
    
    gen::escape_docout::main();


    impl_match::state::from_book::first::main();
    impl_match::state::from_book::without_return::main();
    impl_match::state::from_book::with_arg_ref_mut::main();
    impl_match::state::from_book::task_and_enum_field_and_str_and_result::main();
    impl_match::state::from_book::str_and_result::main();
    impl_match::state::from_book::move_self_2_impl::main();
    impl_match::state::from_book::move_self_2_macro_with_one_enum::main();
    impl_match::state::from_book::two_result::main();

    impl_match::state::state_machine::test();

}

#[ignore]
#[test]
fn main_t(){
    println!("---- main_t() ----");
    // gen::state::state_machine::main();
    // impl_match::state::state_machine::main();

}