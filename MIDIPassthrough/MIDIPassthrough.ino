
void setup() {
  Serial.begin(31250);
}

void loop() {
  if (Serial.available()) {        // If anything comes in Serial (USB),
    Serial.write(Serial.read());  // read it and send it out Serial1 (pins 0 & 1)
  }
}
