# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2020 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------

from nautilus_trader.indicators.base.indicator cimport Indicator
from nautilus_trader.indicators.average.moving_average cimport MovingAverage


cdef class AverageTrueRange(Indicator):
    cdef MovingAverage _moving_average
    cdef bint _use_previous
    cdef double _value_floor
    cdef double _previous_close

    cdef readonly int period
    cdef readonly double value


    cpdef void update(self, double high, double low, double close)
    cpdef void update_mid(self, double close)
    cdef void _floor_value(self)
    cdef void _check_initialized(self)
    cpdef void reset(self)