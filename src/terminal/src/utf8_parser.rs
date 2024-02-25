#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum State {
    ReadingHeader,
    ReadingBody,
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ParserError {
    Pending,
    InvalidBodyByte,
}

#[derive(Clone,Copy,Debug)]
pub struct Parser {
    data: u32,
    curr_bit: usize,
    curr_point: usize,
    total_points: usize,
    state: State,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            data: 0,
            curr_point: 0,
            curr_bit: 0,
            total_points: 0,
            state: State::ReadingHeader,
        }
    }
}

impl Parser {
    pub fn parse_header_byte(&mut self, b: u8) -> bool {
        assert!(self.state == State::ReadingHeader);
        if b & 0b1110_0000 == 0b1100_0000 {
            self.set_total_points(2);
            self.push_code_point(b, 5);
            self.state = State::ReadingBody;
            true
        } else if b & 0b1111_0000 == 0b1110_0000 {
            self.set_total_points(3);
            self.push_code_point(b, 4);
            self.state = State::ReadingBody;
            true
        } else if b & 0b1111_1000 == 0b1111_0000 {
            self.set_total_points(4);
            self.push_code_point(b, 3);
            self.state = State::ReadingBody;
            true
        } else {
            false
        }
    }

    pub fn parse_body_byte(&mut self, b: u8) -> Result<char, ParserError> {
        assert!(self.state == State::ReadingBody);
        if b & 0b1100_0000 != 0b1000_0000 {
            self.state = State::ReadingHeader;
            return Err(ParserError::InvalidBodyByte);
        }
        self.push_code_point(b, 6);
        if !self.is_complete() {
            return Err(ParserError::Pending);
        }
        let c = self.get_char();
        self.state = State::ReadingHeader;
        Ok(c)
    }


    fn push_code_point(&mut self, byte: u8, total_bits: usize) {
        let mask: u8 = 0xFFu8 >> (8-total_bits);
        let bits: u8 = byte & mask;
        let shift_amount: usize = 32-self.curr_bit-total_bits;
        self.data |= (bits as u32) << shift_amount;
        self.curr_point += 1;
        self.curr_bit += total_bits;
    }

    fn is_complete(&self) -> bool {
        self.total_points == self.curr_point
    }

    fn get_char(&self) -> char {
        let shift_amount: usize = 32-self.curr_bit;
        let data = self.data >> shift_amount;
        unsafe { char::from_u32_unchecked(data) }
    }

    fn set_total_points(&mut self, total_points: usize) {
        self.data = 0;
        self.curr_bit = 0;
        self.curr_point = 0;
        self.total_points = total_points;
    }
}
