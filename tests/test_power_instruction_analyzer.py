# SPDX-License-Identifier: LGPL-2.1-or-later
# See Notices.txt for copyright information

import unittest
import power_instruction_analyzer as pia


class TestOverflowFlags(unittest.TestCase):
    def test_text_signature(self):
        self.assertEqual(pia.OverflowFlags.__text_signature__,
                         "(overflow, overflow32)")

    def test_fields(self):
        v = pia.OverflowFlags(overflow=False, overflow32=True)
        self.assertEqual(v.overflow, False)
        self.assertEqual(v.overflow32, True)
        v.overflow = True
        self.assertEqual(v.overflow, True)
        v.overflow32 = False
        self.assertEqual(v.overflow32, False)

    def test_str_repr(self):
        v = pia.OverflowFlags(overflow=False, overflow32=True)
        self.assertEqual(str(v),
                         '{"overflow":false,"overflow32":true}')
        self.assertEqual(repr(v),
                         "OverflowFlags(overflow=False, overflow32=True)")


class TestDivInput(unittest.TestCase):
    def test_text_signature(self):
        self.assertEqual(pia.DivInput.__text_signature__,
                         "(dividend, divisor, result_prev)")

    def test_fields(self):
        v = pia.DivInput(dividend=123, divisor=456, result_prev=789)
        self.assertEqual(v.dividend, 123)
        self.assertEqual(v.divisor, 456)
        self.assertEqual(v.result_prev, 789)
        v.dividend = 1234
        self.assertEqual(v.dividend, 1234)
        v.divisor = 4567
        self.assertEqual(v.divisor, 4567)
        v.result_prev = 7890
        self.assertEqual(v.result_prev, 7890)

    def test_str_repr(self):
        v = pia.DivInput(dividend=123, divisor=456, result_prev=789)
        self.assertEqual(str(v),
                         '{"dividend":"0x7B","divisor":"0x1C8","result_prev":"0x315"}')
        self.assertEqual(repr(v),
                         "DivInput(dividend=123, divisor=456, result_prev=789)")


class TestDivResult(unittest.TestCase):
    def test_text_signature(self):
        self.assertEqual(pia.DivResult.__text_signature__,
                         "(result, overflow)")

    def test_fields(self):
        v = pia.DivResult(result=1234,
                          overflow=pia.OverflowFlags(overflow=False, overflow32=True))
        self.assertEqual(v.result, 1234)
        self.assertIsNotNone(v.overflow)
        self.assertEqual(v.overflow.overflow, False)
        self.assertEqual(v.overflow.overflow32, True)
        v.result = 123
        self.assertEqual(v.result, 123)
        v.overflow = None
        self.assertIsNone(v.overflow)

    def test_str_repr(self):
        v = pia.DivResult(result=1234,
                          overflow=pia.OverflowFlags(overflow=False, overflow32=True))
        self.assertEqual(str(v),
                         '{"result":"0x4D2","overflow":false,"overflow32":true}')
        self.assertEqual(repr(v),
                         "DivResult(result=1234, overflow=OverflowFlags(overflow=False, overflow32=True))")


class TestDivInstrs(unittest.TestCase):
    cases = [
        ("divdeo", '{"result":"0x0","overflow":true,"overflow32":true}'),
        ("divdeuo", '{"result":"0x0","overflow":true,"overflow32":true}'),
        ("divdo", '{"result":"0x36","overflow":false,"overflow32":false}'),
        ("divduo", '{"result":"0x36","overflow":false,"overflow32":false}'),
        ("divweo", '{"result":"0x0","overflow":true,"overflow32":true}'),
        ("divweuo", '{"result":"0x0","overflow":true,"overflow32":true}'),
        ("divwo", '{"result":"0x36","overflow":false,"overflow32":false}'),
        ("divwuo", '{"result":"0x36","overflow":false,"overflow32":false}'),
        ("modsd", '{"result":"0x10"}'),
        ("modud", '{"result":"0x10"}'),
        ("modsw", '{"result":"0x10"}'),
        ("moduw", '{"result":"0x10"}'),
    ]

    def test(self):
        v = pia.DivInput(dividend=0x1234, divisor=0x56, result_prev=0x789)
        for fn_name, expected in self.cases:
            with self.subTest(fn_name=fn_name):
                fn = getattr(pia, fn_name)
                results = fn(v)
                self.assertEqual(str(results), expected)


if __name__ == "__main__":
    unittest.main()
