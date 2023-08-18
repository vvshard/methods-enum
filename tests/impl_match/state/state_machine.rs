//Solving the "Finite State Machine" task from http://rosettacode.org/wiki/Finite_state_machine
//
// For abstraction, it is desirable to implement the transitions of the state machine through its methods:

methods_enum::impl_match!{
enum State {
    Ready:      set() { println!("Ready: d - deposit / q - quit") }
                input_char(ch) { match ch { 
                    'd' => *self = State::Waiting,
                    'q' => *self = State::Exit,
                    _ => (),
                }},
    Waiting:    set() { println!("Waiting: s - select / r - refund") }
                input_char(ch) { match ch { 
                    's' => *self = State::Dispense,
                    'r' => *self = State::Refunding,
                    _ => (),
                }},
    Dispense:   set() { println!("Dispense: r - remove ") }
                input_char(ch) { if ch == 'r' { *self = State::Ready } }
                ,
    Refunding:  set() { 
                    println!("Refunding: refund of the deposit...");
                    *self = State::Ready;
                    self.set();
                },
    Exit:       set() { println!("Exit: goodbye!") }
}
impl State {
    pub fn set(&mut self)                   ~{ match *self }
    pub fn input_char(&mut self, ch: char)  ~{ match *self { return }; self.set() }
}
} // impl_match!

#[allow(unused)]
pub fn main() {
    let mut machine = State::Ready;
    machine.set();

    while !matches!(&machine, State::Exit) {
        let input_line = std::io::stdin().lines().next().unwrap().unwrap_or_default();
        machine.input_char(input_line.chars().next().unwrap_or('\x0d'));
    }
}

pub fn test() {
    let mut machine = State::Ready;
    machine.set();
    assert!(matches!(&machine, State::Ready));
    machine.input_char('d');
    assert!(matches!(&machine, State::Waiting));
    machine.input_char('r');
    assert!(matches!(&machine, State::Ready));
    machine.input_char('d');
    assert!(matches!(&machine, State::Waiting));
    machine.input_char('s');
    assert!(matches!(&machine, State::Dispense));
    machine.input_char('r');
    assert!(matches!(&machine, State::Ready));
    machine.input_char('q');
    assert!(matches!(&machine, State::Exit));
}
