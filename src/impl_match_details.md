### Covered and processed code

The macro handles one `enum` - first in order with @enum priority and unfinished match-expressions in all `~`-marked method bodies of all impl-blocks of one, first of impl struct or enum in the covered code.   
Methods marked with `~` with a missing top-level unfinished match-expression (without `=>`) are passed to the compiler unchanged and without `~`.

The relative position of `enum` and `impl` is not important. The processed `enum` itself can also be the item of the processed `impl`. Impl-blocks are processed both personal and `impl (Trait) for`.

All other code covered by the macro is passed to the compiler unchanged, as if it were outside the macro.

### Ufinished match-expressions

Only one, first in order, incomplete match-expression (without `=>`) is processed at the top level of each method body marked with `~`.

The input expression after the `match` keyword must be of the type of the `enum` being processed or its ref.

If an unfinished match-expression ends with `{}` block without `=>`, that block is considered the default match block for all `enum` variants that do not reference the method containing that match-expression.

If an unterminated match-expression does not contain a default match-arm block, it must be the last one in the statement (ie closed with `;`), or the last one in the body of the method.

### Enum declaration with match-arms

As with the standard `enum` declaration, the enum variants must be separated by commas `,`.

After the name of the `enum` variant and its fields (if any), the methods involved in the variant are listed in arbitrary order. For each method processed, is specified in the following order:
- method name: without fn, pub, generic types, and output value type, but with parentheses after the name `()`, in which indicate the names of the method parameters in the form as when calling (without types and without self).
- the name of Trait for cases when the Trait method is implemented. The Trait name must be specified without the path: use "use" if necessary.
- match-arm block in curly brackets `{}`, which, in fact, will be included in the match-expression of this method.

Method parameter names from `enum` are not passed to the compiler and, generally speaking, can be omitted: parameter names in a match-arm block are semantically and compilably related only to the method signature in the impl-block being processed. But writing them here improves the readability of the blocks, and in the future, perhaps, semantic linking will be added to the macro for them.

For all methods not specified in the `enum` variant: the resulting match-expression for this variant will output the default match-arm block if it is specified in the method's match-expression. Otherwise, no match-arm will be generated for this variant, which will cause a standard compilation error.

Spaces and newlines and regular comments don't matter.

Before the method name, any punctuation is allowed except for `,` in any amount for the purpose of visual emphasis. Usually just `:` after the variant declaration is sufficient.

Attributes and doc-comments before `enum` or its variants will be passed to the compiler unchanged.   
Attributes and doc-comments before method names in `enum` will be ignored.

#### Using enum variants with fields

If field data is used in the match-arm block of a variant, before the method name, you must specify the template for decomposing fields into match-arm block variables in the same form as it will be specified in the match-expression.

The decomposition pattern propagates to subsequent methods of the same `enum` variant, but it can be overridden on any method.   
In the example above, decomposition templates are reassigned to ignore unused fields. Otherwise, to prevent the compiler from reporting `unused`, one would either have to assign `#[allow(unused)]` to the impl block, or use variable names prefixed with _.

#### @-escaping `enum` re-declaration

@-escaping is performed when it is required to describe in the macro match-arms of enum variants for `enum` declared elsewhere in the code (for example, in another module, another `impl_match!` macro, or declared separately because match-arm blocks sizes prevent displaying an `enum` declaration on one screen).   
Example: `@enum State { ...`   
In this case, the macro works with match-arms as described above, but the declaration of the `enum` itself will not be passed to the compiler from the macro.   
If you wish to specify `enum` attributes or `pub` here, `@` must precede them so that they are ignored to be passed to the compiler along with the `enum` declaration.

An `enum` escaped with `@` takes precedence for processing by a macro over other `enum` declarations in the same macro, regardless of order.

### Compiler messages, IDE semantics, and debugging flags

As previously reported, the compiler and IDE[^rust_analyzer] work flawlessly with identifiers included in the resulting method code, i.e. match-arms blocks and decomposition patterns in `enum` variants.

The behavior of macro identifiers (other than simply ignored variant method parameter names) that are not portable from `enum` to the resulting code, such as method and trait names, differs depending on the mode: release-mode or dev-mode, and for the latter - also in depending on debug flags.

#### In release-mode

If a macro finds a mismatch between method and traits names in enum variants with signatures in impl blocks, it will generate a compilation error with a corresponding message.

#### In dev-mode without debugging flags

The macro will create a hidden empty module with identifiers spanned with the names of methods and traits from the `enum` variants, thus connecting them to the standard semantic analysis of the compiler and IDE.   
The macro also performs its own search for inconsistencies, but instead of a compilation error, it only prints a message to the console during the commands `cargo build`/`run`/`test`.

##### This has the following advantages for method names and trait names from `enum` variants:
- almost complete IDE support: highlighting specific errors and semantic links, tooltips, jump to definition, group semantic renaming
- the possibility of the "inline macro" command and in cases of partial reading of methods in the `enum` variants by the macro

##### Currently, this mode has the following non-critical restrictions:
- `enum`, impl object and traits must qualify in macro scope without paths: make appropriate `use` declarations if necessary.
- methods with generics (eg: `mark_obj` from the Shape example) do not support semantic connections: only errors in the name are highlighted.

#### Debug Flags

They can be placed through spaces in parentheses at the very beginning of the macro,   
eg: `impl_match! { (ns ) `...
- flag `ns` or `sn` in any case - replaces the semantic binding of the names of methods and traits in `enum` variants with a compilation error if they are incorrectly specified. Thus, the macro is brought to the behavior as in release-mode. This is worth doing if the IDE does not support proc-macros, or if you want to output the resulting code from the "inline macro" command without an auxiliary semantic module.  
I do not rule out that in some case it is the auxiliary semantic module that will become the source of failure. In this case, the `ns` flag will remove the helper module along with the bug. If this happens, please kindly report the issue to [github](https://github.com/vvshard/methods-enum/issues).
- flag `!` - causes a compilation error in the same case, but without removing the semantic binding.   
The `!` flag can be used to view errors found by the macro itself rather than by the IDE's semantic analysis without running `cargo build`.

## Links

- [Code examples with `impl_match!` macro](https://github.com/vvshard/methods-enum/tree/master/tests/impl_match).



