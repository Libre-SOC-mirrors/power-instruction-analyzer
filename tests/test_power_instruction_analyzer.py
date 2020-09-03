# SPDX-License-Identifier: LGPL-2.1-or-later
# See Notices.txt for copyright information

import unittest
import power_instruction_analyzer as pia


class TestOverflowFlags(unittest.TestCase):
    def test_text_signature(self):
        self.assertEqual(pia.OverflowFlags.__text_signature__,
                         "(so, ov, ov32)")

    def test_fields(self):
        v = pia.OverflowFlags(so=False, ov=False, ov32=True)
        self.assertEqual(v.so, False)
        self.assertEqual(v.ov, False)
        self.assertEqual(v.ov32, True)
        v.so = True
        self.assertEqual(v.so, True)
        v.ov = True
        self.assertEqual(v.ov, True)
        v.ov32 = False
        self.assertEqual(v.ov32, False)

    def test_str_repr(self):
        v = pia.OverflowFlags(so=False, ov=False, ov32=True)
        self.assertEqual(str(v),
                         '{"so":false,"ov":false,"ov32":true}')
        self.assertEqual(repr(v),
                         "OverflowFlags(so=False, ov=False, ov32=True)")


class TestCarryFlags(unittest.TestCase):
    def test_text_signature(self):
        self.assertEqual(pia.CarryFlags.__text_signature__,
                         "(ca, ca32)")

    def test_fields(self):
        v = pia.CarryFlags(ca=False, ca32=True)
        self.assertEqual(v.ca, False)
        self.assertEqual(v.ca32, True)
        v.ca = True
        self.assertEqual(v.ca, True)
        v.ca32 = False
        self.assertEqual(v.ca32, False)

    def test_str_repr(self):
        v = pia.CarryFlags(ca=False, ca32=True)
        self.assertEqual(str(v),
                         '{"ca":false,"ca32":true}')
        self.assertEqual(repr(v),
                         "CarryFlags(ca=False, ca32=True)")


class TestConditionRegister(unittest.TestCase):
    def test_text_signature(self):
        self.assertEqual(pia.ConditionRegister.__text_signature__,
                         "(lt, gt, eq, so)")

    def test_fields(self):
        v = pia.ConditionRegister(lt=False, gt=True, eq=False, so=True)
        self.assertEqual(v.lt, False)
        self.assertEqual(v.gt, True)
        self.assertEqual(v.eq, False)
        self.assertEqual(v.so, True)
        v.lt = True
        self.assertEqual(v.lt, True)
        v.gt = False
        self.assertEqual(v.gt, False)
        v.eq = True
        self.assertEqual(v.eq, True)
        v.so = False
        self.assertEqual(v.so, False)

    def test_str_repr(self):
        v = pia.ConditionRegister(lt=False, gt=True, eq=False, so=True)
        self.assertEqual(str(v),
                         '{"lt":false,"gt":true,"eq":false,"so":true}')
        self.assertEqual(repr(v),
                         "ConditionRegister(lt=False, gt=True, eq=False, so=True)")


class TestInstructionInput(unittest.TestCase):
    def test_text_signature(self):
        self.assertEqual(pia.InstructionInput.__text_signature__,
                         "(ra, rb, rc)")

    def test_fields(self):
        v = pia.InstructionInput(ra=123, rb=456, rc=789)
        self.assertEqual(v.ra, 123)
        self.assertEqual(v.rb, 456)
        self.assertEqual(v.rc, 789)
        v.ra = 1234
        self.assertEqual(v.ra, 1234)
        v.rb = 4567
        self.assertEqual(v.rb, 4567)
        v.rc = 7890
        self.assertEqual(v.rc, 7890)

    def test_str_repr(self):
        v = pia.InstructionInput(ra=123, rb=456, rc=789)
        self.assertEqual(str(v),
                         '{"ra":"0x7B","rb":"0x1C8","rc":"0x315"}')
        self.assertEqual(repr(v),
                         "InstructionInput(ra=123, rb=456, rc=789)")


class TestInstructionOutput(unittest.TestCase):
    maxDiff = 1000

    def test_text_signature(self):
        self.assertEqual(pia.InstructionOutput.__text_signature__,
                         "(rt=None, overflow=None, carry=None, cr0=None, "
                         "cr1=None, cr2=None, cr3=None, cr4=None, cr5=None, "
                         "cr6=None, cr7=None)")

    def test_fields(self):
        v = pia.InstructionOutput(
            overflow=pia.OverflowFlags(so=False, ov=False, ov32=True))
        self.assertIsNone(v.rt)
        self.assertIsNone(v.carry)
        self.assertIsNotNone(v.overflow)
        self.assertEqual(v.overflow.so, False)
        self.assertEqual(v.overflow.ov, False)
        self.assertEqual(v.overflow.ov32, True)
        self.assertIsNone(v.cr0)
        self.assertIsNone(v.cr1)
        self.assertIsNone(v.cr2)
        self.assertIsNone(v.cr3)
        self.assertIsNone(v.cr4)
        self.assertIsNone(v.cr5)
        self.assertIsNone(v.cr6)
        self.assertIsNone(v.cr7)
        v.rt = 123
        self.assertEqual(v.rt, 123)
        v.overflow = None
        self.assertIsNone(v.overflow)
        v.cr2 = pia.ConditionRegister(lt=False, gt=False, eq=False, so=False)
        self.assertIsNotNone(v.cr2)
        v.carry = pia.CarryFlags(ca=False, ca32=True)
        self.assertIsNotNone(v.carry)
        self.assertEqual(v.carry.ca, False)
        self.assertEqual(v.carry.ca32, True)

    def test_str_repr(self):
        v = pia.InstructionOutput(
            overflow=pia.OverflowFlags(so=False, ov=False, ov32=True),
            carry=pia.CarryFlags(ca=True, ca32=False),
            cr0=pia.ConditionRegister(lt=True, gt=True, eq=True, so=True),
            cr2=pia.ConditionRegister(lt=False, gt=False, eq=False, so=False))
        self.assertEqual(str(v),
                         '{"so":false,"ov":false,"ov32":true,"ca":true,'
                         '"ca32":false,"cr0":{"lt":true,"gt":true,"eq":true,'
                         '"so":true},"cr2":{"lt":false,"gt":false,"eq":false,'
                         '"so":false}}')
        self.assertEqual(repr(v),
                         "InstructionOutput(rt=None, overflow=OverflowFlags("
                         "so=False, ov=False, ov32=True), carry=CarryFlags("
                         "ca=True, ca32=False), cr0=ConditionRegister(lt=True,"
                         " gt=True, eq=True, so=True), cr1=None, "
                         "cr2=ConditionRegister(lt=False, gt=False, eq=False, "
                         "so=False), cr3=None, cr4=None, cr5=None, cr6=None, "
                         "cr7=None)")


class TestDivInstrs(unittest.TestCase):
    def test(self):
        v = pia.InstructionInput(ra=0x1234, rb=0x56, rc=0x789)
        for instr in pia.INSTRS:
            with self.subTest(instr=instr):
                fn_name = instr.replace(".", "_")
                fn = getattr(pia, fn_name)
                self.assertEqual(fn.__text_signature__, "(inputs)")
                results = fn(v)
                self.assertIsInstance(results, pia.InstructionOutput)

    def test_exception(self):
        with self.assertRaisesRegex(ValueError, "missing instruction input"):
            v = pia.InstructionInput(ra=1)
            pia.mulldo_(v)
        with self.assertRaisesRegex(ValueError, "missing instruction input"):
            v = pia.InstructionInput(ra=1, rb=1)
            pia.mulldo_(v)


if __name__ == "__main__":
    unittest.main()
