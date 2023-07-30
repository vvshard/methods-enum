### Covered and processed code

The macro handles one `enum` - first in order with @enum priority and unfinished match-expressions in all `~`-marked method bodies of all impl-blocks of one, first of impl struct or enum in the covered code.   
Methods marked with `~` with a missing top-level unfinished match-expression (without `=>`) are passed to the compiler unchanged and without `~`.

The relative position of `enum` and `impl` is not important. The processed `enum` itself can also be the item of the processed `impl`. Impl-blocks are processed both personal and `impl (Trait) for`.
