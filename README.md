# Attribute macro 'methods_enum::gen'
Based on method signatures, the following are formed enum with options from argument tuples
  and the bodies of those methods, with an argument handler method call from that enum.

This allows the handler method to control the behavior of the methods depending on the context.

There are two syntax options:

1. For the case where methods returning a value have the same return type:

`#[methods_enum::gen(`*EnumName*`: `*handler_name*`)]`

where:
- *EnumName*: the name of the automatically generated enum.
- *handler_name*: handler method name

2. In case of more than one meaningful return type:

`#[methods_enum::gen(`*EnumName*`: `*handler_name*` = `*OutName*`)]`

where:
 - *OutName*: the name of the automatically generated enum with options from single tuples of return types.

In this case, you can also specify default return value expressions.



