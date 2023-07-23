// #![allow(unused)]

mod gen;
mod impl_match;

#[test]
fn test_main() {
    // region: gen

    use gen::state::from_book as gen_from_book;

    gen_from_book::first::main();
    gen_from_book::without_return::main();
    gen_from_book::with_arg_ref_mut::main();
    gen_from_book::task_and_str_and_result::main();
    gen_from_book::str_and_result::main();
    gen_from_book::move_self_2_impl::main();
    gen_from_book::two_result::main();

    gen::state::state_machine::test();

    gen::escape_docout::main();

    // endregion: gen

    // region: impl_match

    use impl_match::state::from_book;

    from_book::first::main();
    from_book::without_return::main();
    from_book::with_arg_ref_mut::main();
    from_book::task_and_enum_field_and_str_and_result::main();
    from_book::str_and_result::main();
    from_book::move_self_2_impl::main();
    from_book::move_self_2_macro_with_one_enum::main();
    from_book::two_result::main();

    impl_match::state::state_machine::test();

    // endregion: impl_match
}

#[ignore]
#[test]
/// Run - from the pop-up menu of the rust-analyzer "Run Test".
fn main_t() {
    println!("---- main_t() ----\n");
    // gen::state::state_machine::main();
    impl_match::state::state_machine::main();
}
