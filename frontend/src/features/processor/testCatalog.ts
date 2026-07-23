export const PROCESSOR_TESTS = [
  {
    id: "system_208v",
    title: "System 208V",
    outputSubfolder: "System_208V_Accuracy_Reports",
  },
  {
    id: "system_415v",
    title: "System 415V",
    outputSubfolder: "System_415V_Accuracy_Reports",
  },
] as const;

export type ProcessorTest = (typeof PROCESSOR_TESTS)[number];
export type ProcessorTestId = ProcessorTest["id"];

export function processorTest(testId: ProcessorTestId): ProcessorTest {
  return PROCESSOR_TESTS.find((test) => test.id === testId) ?? PROCESSOR_TESTS[0];
}
