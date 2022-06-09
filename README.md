# attribute macro 'methods_enum::gen'
By signatures from methods without bodies, are formed:
an enum with options named as methods tuples, corresponding for them arguments,
and bodies for that methods calls for handler method this enum of tuples with parameters.

This allows the handler method to manipulate the behavior of the methods depending on the context.

There are two options syntaxes:

1- For case when methods that return a value have the same return type:

`#[methods_enum::gen(`*EnumName*`: `*handler_name*`)]`

where:
- *EnumName*: The name of the automatically accepted enumeration.
- *handler_name*: name of the handler method

2- For the case of more than one meaningful return type:

`#[methods_enum::gen(`*EnumName*`: `*handler_name*` = `*OutName*`)]`

where - *OutName*: the name of the automatically retrieved enum
with method-named options single-tuples of the return type.

In this case, you can also specify default return value expressions in the method signature.

For more details, see the [module documentation](self)