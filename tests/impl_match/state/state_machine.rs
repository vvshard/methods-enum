//Solving the "Finite State Machine" task from http://rosettacode.org/wiki/Finite_state_machine
//
// For abstraction, it is desirable to implement the transitions of the state machine through its methods:

methods_enum::impl_match!{
enum State {
    Ready:
        set() { println!("Ready: d - deposit / q - quit") }
        input_char(ch) { match ch { 
            'd' => self.set_state(State::Waiting),
            'q' => self.set_state(State::Exit),
            _ => self.set(),
        }},
    Waiting:
        set() { println!("Waiting: s - select / r - refund") }
        input_char(ch) { match ch { 
            's' => self.set_state(State::Dispense),
            'r' => self.set_state(State::Refunding),
            _ => self.set(),
        }},
    Dispense:
        set() { println!("Dispense: r - remove ") }
        input_char(ch) { match ch { 
            'r' => self.set_state(State::Ready),
            _ => self.set(),
        }},
    Refunding: set() { 
            println!("Refunding: refund of the deposit...");
            self.set_state(State::Ready);
        },
    Exit: set() { println!("Exit: goodbye!") }
}
impl State {
    pub fn set(&mut self)                       ~{ match *self }
    pub fn input_char(&mut self, ch: char)      ~{ match *self }

    fn set_state(&mut self, new_state: State) {
        *self = new_state;
        self.set();
    }
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
